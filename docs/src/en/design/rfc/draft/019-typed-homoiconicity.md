---
title: RFC-019: Typed Homoiconicity
---

# RFC-019: Typed Homoiconicity - Syntax as Types

> **Status**: Draft
>
> **Author**: Chen Xu
>
> **Created**: 2026-02-20
>
> **⚠️ Permanent Experimental Declaration**: This is an **exploratory experiment** to verify the feasibility of the language design concept "syntax as types". **This RFC will NEVER be merged**, and will not enter the dev/main branch regardless of the outcome. The experimental branch will be abandoned or archived upon completion.
>
> - **Experiment Goal**: Verify the implementation difficulty and potential value of type-level homoiconicity
> - **Stop Loss Line**: Abandon after 6 months without progress
> - **Success Criteria**: Successfully run at least one user-defined keyword (complete parsing → compilation → execution)
>
> **No guarantee of merging to the main branch**, and may be rejected or abandoned for various reasons in the future. Do not use this feature in production environments.

---

## Abstract

This RFC proposes an aggressive language design experiment: **making the language's syntactic structure itself part of the type system**.

The core idea originates from Lisp's "code as data" (homoiconicity), but implemented through a **static type system**:
- Abstract syntax trees (AST) are types
- Keywords are predefined instances of types
- Users can extend language syntax by defining types

This means: the language itself becomes a composable, extensible set of "building blocks".

---

## Motivation

### Why Conduct This Experiment?

1. **Pursuit of Unification**: Eliminate the special syntactic element of "keywords", making everything types and functions
2. **Language Extensibility**: Users can define new syntactic structures just like defining functions
3. **Type-Safe Macros**: Traditional macros (text substitution) are dangerous; type-level homoiconicity can provide compile-time checks
4. **Learning Purpose**: Deeply understand the essence of language design

### Relationship with Lisp

Lisp has long implemented "code as data":
```lisp
; Lisp code itself is an S-expression
(if (> x 0) "positive" "negative")
```

The difference in this experiment is: **strengthening this concept with a static type system**.

---

## Proposal

### Core Concepts

#### 1. Syntax Tree as Type (AST as Type)

```yaoxiang
// Syntax tree nodes are all types
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

#### 2. Keywords = Functions That Process Types

```yaoxiang
// Evaluators are functions that process these types
eval_if: (node: If, env: Env) -> Value = ...
eval_while: (node: While, env: Env) -> Value = ...
eval_return: (node: Return, env: Env) -> Value = ...
eval_block: (node: Block, env: Env) -> Value = ...

// Compilers can also be functions
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. Types Carry Parsing Rules (Core Innovation)

This is the key to this experiment: **types not only describe data but also carry rules for how to parse code**.

```yaoxiang
// Syntax rule type
SyntaxRule: Type = {
    // How to parse code of this type
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // How to compile/evaluate instances of this type
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
        // Transform to a while loop
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
│                   Source Code                       │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│               Parser                                │
│  - Recognize keywords                               │
│  - Find the corresponding SyntaxRule type          │
│  - Call the type's parse method                    │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (Type Instances)                   │
│  If, While, Match, TimesLoop...                    │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│           Compiler / Interpreter                    │
│  - Call the type's compile/eval methods             │
│  - Generate target code or execute                 │
└─────────────────────────────────────────────────────┘
```

### Key Technical Issues

#### 1. Control Flow as Functions

Problem: `if` needs to evaluate only one branch, which cannot be done with ordinary function calls.

Solution: Pass thunks (lazy evaluation)

```yaoxiang
// Internal representation after compilation
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk, lazy evaluation
    else: () -> Value
}
```

#### 2. Non-Local Return of `return`

Problem: `return` needs to exit from multiple function levels.

Solutions:
- Option A: Compile-time CPS transformation
- Option B: Use Result/Either monad
- Option C: Limit the scope of `return`

#### 3. Syntactic Ambiguity

Problem: How to distinguish `if(x > 0) { 1 }` as a function call vs. a keyword?

Solution:
- Keywords use special syntax (e.g., `if ... { } else { }`)
- Or constrain through the type system

#### 4. Infinite Recursion

Problem: Users may define self-referential syntax rules.

Solution: Detect circular dependencies at compile time

---

## Relationship with Existing Systems

### Relationship with RFC-010 (Unified Type Syntax)

RFC-010 implemented the unified syntax `name: type = value`; this RFC is its extension:

| RFC-010 | This RFC |
|----------|----------|
| Variables, functions, types are all `name: type = value` | Keywords are also `name: type = value` |
| Types are values | Syntax rules are also values |
| `Type` is a meta type | `SyntaxRule` is the meta type for syntax |

### Comparison with Lisp/Macros

| Feature | Lisp Macros | This Experiment |
|---------|-------------|-----------------|
| Code representation | S-expression (lists) | Type instances |
| Extension method | defmacro | Define SyntaxRule types |
| Type safety | Weak (text substitution) | Strong (type checking) |
| Parsing time | Runtime/compile-time | Compile-time |
| IDE support | Weak | Strong (type information) |

---

## Branch Plan

### Experimental Branch

```
Branch name: exp/typed-homoiconicity
Created from dev branch
```

**Important**:
- This is an **experimental branch** and will not be frequently merged with dev
- May be developed independently for a long time
- **No guarantee of merging to main**
- If the experiment fails, the branch will be abandoned

### Development Phases

> **⚠️ Experiment Time Limit: 6 months**

| Phase | Goal | Expected Time | Notes |
|------|------|---------------|-------|
| Phase 1 | Proof of concept: Implement AST types with existing syntax | 2 weeks | |
| Phase 2 | Implement basic evaluator | 2 weeks | Key challenge: if/return control flow |
| Phase 3 | Implement SyntaxRule type parsing rules | 3 weeks | |
| Phase 4 | User-defined syntax extensions | 3 weeks | Core goal: successfully run at least one custom keyword |
| Phase 5 | Optimization and documentation | 2 weeks | Experiment concludes |

**Timeout Handling**: If Phase 2 (control flow implementation) shows no progress after 4 weeks, consider abandonment.

---

## Trade-offs

### Advantages

- **Ultimate Unification**: Eliminates the boundary between keywords and ordinary code
- **Language Extensibility**: Users can define their own syntax
- **Type Safety**: Safer than traditional macros
- **Learning Value**: Deeply understand language fundamentals

### Disadvantages

- **Implementation Complexity**: Requires significant compiler modifications
- **Performance Concerns**: Runtime interpretation may be slow
- **Learning Curve**: Abstract concepts, requires understanding the type system
- **Practicality Question**: Possibly over-engineered

### Risks

- Experiment may fail, unable to find practical use cases
- Implementation difficulty exceeds expectations
- Conflicts with existing features

---

## Open Questions

- [ ] How to handle syntax conflicts (user-defined rules vs. built-in)?
- [ ] Performance optimization plan?
- [ ] Is a syntax import/export mechanism needed?
- [ ] How to integrate with the existing module system?

---

## Appendix

### Glossary

| Term | Definition |
|------|------------|
| Homoiconicity | Code and data use the same representation |
| AST | Abstract Syntax Tree, the abstract representation of a program |
| SyntaxRule | A type that carries syntax parsing rules |
| Thunk | A function wrapper for lazy evaluation |
| CPS | Continuation Passing Style |

### References

- [Lisp Wiki: Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
- [Julia Metaprogramming](https://docs.julialang.org/en/v1/manual/metaprogramming/)
- [Rust Procedural Macros](https://doc.rust-lang.org/book/ch19-06-macros.html)

---

## Lifecycle and Destiny

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
       ⚠️ Permanent Experimental Branch (exp/typed-homoiconicity)

       Possible outcomes:
       ├─► Successful validation → Archive, never merge
       ├─► Failure → Abandon branch
       └─► Timeout → Give up and abandon

       ⚠️ Regardless of outcome, this RFC will NEVER be merged
```

> **⚠️ Important Reminder**: This is an exploratory experiment, **will never be merged**. Please do not depend on this feature in production code.