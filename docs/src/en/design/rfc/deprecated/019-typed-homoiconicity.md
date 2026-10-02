---
title: 'RFC-019: Typed Homoiconicity - Syntax as Types'
status: 'Deprecated'
author: 'Chenxu'
created: '2026-02-20'
updated: '2026-10-02'
---

# RFC-019: Typed Homoiconicity - Syntax as Types

> **⚠️ Stop-Loss Resolution (2026-10-02)**: This RFC was created on 2026-02-20, with a stop-loss
> line declared in the main text as "abandon if no progress in 6 months", i.e., due on 2026-08-20.
> Upon expiry, the success criterion (running through the full parse→compile→execute pipeline of at
> least one user-defined keyword) was **not met**. The last substantive change was on 2026-06-05
> (only a site-wide formatting unification followed on 2026-07-28). Archived per the stop-loss line:
> status changed to "Deprecated", the file moved to `rfc/deprecated/`, retained only for historical
> reference.
>
> **⚠️ Permanent Experimental Declaration**: This is an **exploratory experiment** to verify the
> feasibility of the "syntax as types" language design philosophy. **This RFC will never be
> merged**, regardless of the outcome, into the dev/main branches. The experimental branch will be
> deprecated or archived after completion.
>
> - **Experiment Goal**: Verify the implementation difficulty and potential value of typed
>   homoiconicity
> - **Stop-Loss Line**: Abandon if no progress in 6 months
> - **Success Criterion**: Run through at least one user-defined keyword (full parse→compile→execute
>   pipeline)
>
> **There is no guarantee of merging into the main branch**, and it may be rejected or abandoned in
> the future for various reasons. Please do not use this feature in production environments.
>
> **⚠️ Positioning Note**: This RFC is a language design thought experiment and does not provide an
> engineering solution. For practical extensible parser patterns, see Rust's `syn::Parse` or
> Haskell's `parsec`.

---

## Summary

This RFC proposes a radical language design experiment: **let the syntactic structure of the
language itself become part of the type system**.

The core idea is derived from Lisp's "code as data" (homoiconicity), but realized through a **static
type system**:

- The syntax tree (AST) is a type
- Keywords are predefined instances of types
- Users can extend the language syntax by defining types

This means: the language itself becomes composable, extensible "building blocks".

---

## Motivation

### Why do this experiment?

1. **Pursuit of Uniformity**: Eliminate the special syntactic element "keyword", so that everything
   is types and functions
2. **Language Extensibility**: Users can define new syntactic structures just like defining
   functions
3. **Type-Safe Macros**: Traditional macros (text substitution) are dangerous; typed homoiconicity
   can provide compile-time checking
4. **Learning Purpose**: Deeply understand the essence of language design

### Relationship with Lisp

Lisp already implements "code as data":

```lisp
; Lisp code is itself an S-expression
(if (> x 0) "positive" "negative")
```

The difference in this experiment is: **reinforcing this idea with a static type system**.

---

## Proposal

### Core Concepts

#### 1. AST as Type

```yaoxiang
// AST nodes are all types
If: Type = { condition: Expr, then: Block, else: Block }
While: Type = { condition: Expr, body: Block }
Return: Type = { value: Expr }
Block: Type = { statements: Array[Expr] }
Let: Type = { name: String, value: Expr, body: Expr }
Function: Type = { params: Array[Param], body: Expr }
Call: Type = { func: Expr, args: Array[Expr] }

// Basic types
Literal: Type = { value: Int }
StringLiteral: Type = { value: String }
Variable: Type = { name: String }
```

#### 2. Keywords = Functions that Handle Types

```yaoxiang
// Evaluators are functions that handle these types
eval_if: (node: If, env: Env) -> Value = ...
eval_while: (node: While, env: Env) -> Value = ...
eval_return: (node: Return, env: Env) -> Value = ...
eval_block: (node: Block, env: Env) -> Value = ...

// Compilers can also be functions
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. Types Carry Parsing Rules (Core Innovation)

This is the key to this experiment: **a type not only describes data, but also carries the rules for
how to parse code**.

```yaoxiang
// Syntax rule type
SyntaxRule: Type = {
    // How to parse code of this type
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // How to compile/evaluate type instances
    compile: (node: Self, ctx: CompileContext) -> IR
    eval: (node: Self, env: Env) -> Value
}

// Syntax rules for the IF type
IF: SyntaxRule = {
    // Parse "if (cond) { then } else { else }"
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("if")
        cond = parse_expression(tokens)
        consume("{")
        then_block = parse_block(tokens)
        consume("}")
        consume("else")
        consume("{")
        else_block = parse_block(tokens)
        consume("}")
        return If(cond, then_block, else_block), tokens
    }

    eval: (node: If, env: Env) -> Value = {
        if eval(node.condition, env) != 0 {
            return eval(node.then, env)
        } else {
            return eval(node.else, env)
        }
    }
}
```

#### 4. User-Defined Syntax Extensions

Users can define their own "keywords":

```yaoxiang
// User defines a new syntactic structure: unless
Unless: SyntaxRule = {
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("unless")
        cond = parse_expression(tokens)
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // unless is equivalent to if (!cond) { body }
        return If(Not(cond), body, Block([])), tokens
    }
}

// Usage
unless x > 0 {
    print("x is not positive")
}

// Expands to
if !(x > 0) {
    print("x is not positive")
}
```

### Examples

#### Complete Example: Custom Loop Syntax

```yaoxiang
// Define a "times" loop: n.times { ... } runs n times
TimesLoop: SyntaxRule = {
    parse: (tokens: TokenStream) -> (While, remaining) = {
        receiver = parse_expression(tokens)  // Get the number
        consume(".times")
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // Convert to a while loop
        counter_var = gensym("i")
        return While(
            Less(Variable(counter_var), receiver),
            Block([
                body,
                Assign(counter_var, Add(Variable(counter_var), Literal(1)))
            ])
        ), tokens
    }
}

// Usage
5.times {
    print("Hello!")
}

// Expands to
i = 0
while i < 5 {
    print("Hello!")
    i = i + 1
}
```

#### Example: Pattern Matching Syntax

```yaoxiang
// User-defined pattern matching
Match: SyntaxRule = {
    parse: (tokens: TokenStream) -> (MatchNode, remaining) = {
        subject = parse_expression(tokens)
        consume("{")
        cases = []
        while !check("}") {
            pattern = parse_pattern(tokens)
            consume("=>")
            body = parse_expression(tokens)
            cases.push((pattern, body))
        }
        consume("}")
        return MatchNode(subject, cases), tokens
    }
}

// Usage
match x {
    0 => "zero",
    1 => "one",
    n if n > 10 => "big",
    _ => "other"
}
```

---

## Detailed Design

### System Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Source Code                       │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              Syntax Parser (Parser)                  │
│  - Recognize keywords                                │
│  - Find the corresponding SyntaxRule type            │
│  - Call the type's parse method                      │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (Type Instances)                    │
│  If, While, Match, TimesLoop...                     │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              Compiler/Interpreter                    │
│  - Call the type's compile/eval method               │
│  - Generate target code or execute                   │
└─────────────────────────────────────────────────────┘
```

### Key Technical Issues

#### 1. Control Flow Functionalization

Problem: `if` needs to evaluate only one branch and cannot be a normal function call.

Solution: Pass in thunks (lazy evaluation)

```yaoxiang
// Compiled internal representation
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk, lazy evaluation
    else: () -> Value
}
```

#### 2. Non-Local Return of `return`

Problem: `return` needs to jump out of multiple layers of functions.

Solution:

- Option A: Compile-time CPS transformation
- Option B: Use Result/Either monad
- Option C: Restrict the scope of `return`

#### 3. Syntactic Ambiguity

Problem: How to distinguish whether `if(x > 0) { 1 }` is a function call or a keyword?

Solution:

- Keywords use special syntax (e.g., `if ... { } else { }`)
- Or constrain through the type system

#### 4. Infinite Recursion

Problem: Users may define self-referential syntax rules.

Solution: Detect circular dependencies at compile time

---

## Relationship with Existing Systems

### Relationship with RFC-010 (Unified Type Syntax)

RFC-010 implements the unified syntax `name: type = value`; this RFC is its extension:

| RFC-010                                                  | This RFC                                |
| -------------------------------------------------------- | --------------------------------------- |
| Variables, functions, types are all `name: type = value` | Keywords are also `name: type = value`  |
| Types are values                                         | Syntax rules are also values            |
| `Type` is a meta type                                    | `SyntaxRule` is the meta type of syntax |

### Comparison with Lisp/Macros

| Feature             | Lisp Macros              | This Experiment           |
| ------------------- | ------------------------ | ------------------------- |
| Code Representation | S-expression (list)      | Type instance             |
| Extension Method    | defmacro                 | Define SyntaxRule type    |
| Type Safety         | Weak (text substitution) | Strong (type checking)    |
| Parsing Time        | Runtime/Compile-time     | Compile-time              |
| IDE Support         | Weak                     | Strong (type information) |

---

## Branch Plan

### Experimental Branch

```
Branch name: exp/typed-homoiconicity
Created from the dev branch
```

**Important**:

- This is an **experimental branch** and will not be frequently merged with dev
- It may be developed independently for a long time
- **There is no guarantee of merging into main**
- If the experiment fails, the branch will be deprecated

### Development Phases

> **⚠️ Experiment Time Cap: 6 months**

| Phase   | Goal                                                       | Expected Time | Notes                                              |
| ------- | ---------------------------------------------------------- | ------------- | -------------------------------------------------- |
| Phase 1 | Proof of concept: implement AST types with existing syntax | 2 weeks       |                                                    |
| Phase 2 | Implement a basic evaluator                                | 2 weeks       | Key challenge: if/return control flow              |
| Phase 3 | Implement parsing rules for the SyntaxRule type            | 3 weeks       |                                                    |
| Phase 4 | User-defined syntax extensions                             | 3 weeks       | Core goal: run through at least one custom keyword |
| Phase 5 | Optimization and documentation                             | 2 weeks       | Experiment ends                                    |

**Overtime Handling**: If Phase 2 (control flow implementation) shows no progress for more than 4
weeks, abandonment should be considered.

---

## Trade-offs

### Advantages

- **Ultimate Uniformity**: Eliminate the boundary between keywords and ordinary code
- **Language Extensibility**: Users can define their own syntax
- **Type Safety**: Safer than traditional macros
- **Learning Value**: Deeply understand the essence of language

### Disadvantages

- **Implementation Complexity**: Requires major compiler modifications
- **Performance Concerns**: Runtime interpretation may be slow
- **Learning Curve**: Abstract concepts requiring understanding of the type system
- **Practicality Questionable**: May be over-engineered

### Risks

- The experiment may fail, finding no practical use case
- Implementation difficulty exceeds expectations
- Conflicts with existing features

---

## Open Questions

- [ ] How to handle syntax conflicts (user-defined rules conflicting with built-ins)?
- [ ] Performance optimization plan?
- [ ] Is a syntax import/export mechanism needed?
- [ ] How to integrate with the existing module system?

---

## Appendix

### Glossary

| Term                       | Definition                                           |
| -------------------------- | ---------------------------------------------------- |
| Homoiconicity              | Code and data use the same representation            |
| AST (Abstract Syntax Tree) | The abstract syntax tree representation of a program |
| SyntaxRule                 | A type carrying syntax parsing rules                 |
| Thunk                      | A function wrapper for lazy evaluation               |
| CPS                        | Continuation Passing Style                           |

### References

- [Wikipedia: Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
- [Julia Metaprogramming](https://docs.julialang.org/en/v1/manual/metaprogramming/)
- [Rust Procedural Macros](https://doc.rust-lang.org/book/ch19-06-macros.html)

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
       ⚠️ Permanent experimental branch (exp/typed-homoiconicity)

       Possible outcomes:
       ├─► Successful verification → Archive, never merge
       ├─► Failure → Deprecate branch
       └─► Timeout → Abandon and deprecate

       ⚠️ Regardless of the outcome, this RFC will never be merged
```

> **⚠️ Important Reminder**: This is an exploratory experiment, **never to be merged**. Please do
> not rely on this feature in production code.
