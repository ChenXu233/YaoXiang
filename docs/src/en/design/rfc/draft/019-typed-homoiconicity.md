---
title: "RFC-019: Typed Homoiconicity"
---

# RFC-019: Typed Homoiconicity - Syntax as Type

> **Status**: Draft
>
> **Author**: 晨煦
>
> **Created**: 2026-02-20
>
> **⚠️ Permanent Experimental Notice**: This is an **exploratory experiment** to verify the feasibility of "syntax as type" language design concept. **This RFC will NEVER merge**, regardless of outcomes, it will never enter dev/main branch. The experimental branch will be abandoned or archived after completion.
>
> - **Experiment Goal**: Verify implementation difficulty and potential value of typed homoiconicity
> - **Time Limit**: 6 months without progress = abort
> - **Success Criteria**: At least one user-defined keyword can run end-to-end (parse → compile → execute)
>
> **No guarantee it will merge to main branch**, may be rejected or abandoned for various reasons. Do not use this feature in production environments.

---

## Abstract

This RFC proposes a radical language design experiment: **making the language's syntax structure part of the type system itself**.

The core idea originates from Lisp's "code as data" (homoiconicity), but implemented through a **static type system**:
- Syntax trees (AST) are types
- Keywords are predefined instances of types
- Users can extend language syntax by defining types

This means: the language itself becomes composable, extensible "building blocks".

---

## Motivation

### Why This Experiment?

1. **Pursuit of Unity**: Eliminate "keywords" as special syntax elements, making everything types and functions
2. **Language Extensibility**: Users can define new syntax structures like defining functions
3. **Type-Safe Macros**: Traditional macros (text substitution) are dangerous; typed homoiconicity provides compile-time checking
4. **Learning Purpose**: Deeply understand the essence of language design

### Relationship with Lisp

Lisp has long implemented "code as data":
```lisp
; Lisp code itself is S-expression
(if (> x 0) "positive" "negative")
```

The difference of this experiment: **strengthening this idea with a static type system**.

---

## Proposal

### Core Concepts

#### 1. AST as Type

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

#### 2. Keywords = Functions Processing Types

```yaoxiang
// Evaluators are functions processing these types
eval_if: (node: If, env: Env) -> Value = ...
eval_while: (node: While, env: Env) -> Value = ...
eval_return: (node: Return, env: Env) -> Value = ...
eval_block: (node: Block, env: Env) -> Value = ...

// Compilers can also be functions
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. Types Carry Parsing Rules (Core Innovation)

This is the key: **types not only describe data, but also carry rules for parsing code**.

```yaoxiang
// Syntax rule type
SyntaxRule: Type = {
    // How to parse code of this type
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // How to compile/evaluate type instances
    compile: (node: Self, ctx: CompileContext) -> IR
    eval: (node: Self, env: Env) -> Value
}

// Syntax rule for IF type
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
// User defines a new syntax structure: unless
Unless: SyntaxRule = {
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("unless")
        cond = parse_expression(tokens)
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // unless equals if (!cond) { body }
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
        receiver = parse_expression(tokens)  // get the number
        consume(".times")
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // Convert to while loop
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
// User defines pattern matching
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
─┐
│┌────────────────────────────────────────────────────              Parser                                  │
│  - Recognize keywords                               │
│  - Find corresponding SyntaxRule type               │
│  - Call type's parse method                         │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (Type Instances)                    │
│  If, While, Match, TimesLoop...                    │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              Compiler/Interpreter                   │
│  - Call type's compile/eval methods                │
│  - Generate target code or execute                  │
└─────────────────────────────────────────────────────┘
```

### Key Technical Issues

#### 1. Control Flow Functionization

Problem: `if` needs to evaluate only one branch, cannot use ordinary function calls.

Solution: Pass thunks (delayed evaluation)

```yaoxiang
// Internal representation after compilation
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk, delayed evaluation
    else: () -> Value
}
```

#### 2. Non-local Return of `return`

Problem: `return` needs to exit from multiple nested functions.

Solutions:
- A: Compile-time CPS transformation
- B: Use Result/Either monad
- C: Limit `return` scope

#### 3. Syntax Ambiguity

Problem: How to distinguish `if(x > 0) { 1 }` as function call or keyword?

Solution:
- Keywords use special syntax (like `if ... { } else { }`)
- Or constrain through type system

#### 4. Infinite Recursion

Problem: Users may define self-referential syntax rules.

Solution: Detect circular dependencies at compile time

---

## Relationship with Existing Systems

### Relationship with RFC-010 (Unified Type Syntax)

RFC-010 implements unified `name: type = value` syntax; this RFC extends it:

| RFC-010 | This RFC |
|----------|----------|
| Variables, functions, types are all `name: type = value` | Keywords are also `name: type = value` |
| Types are values | Syntax rules are also values |
| `Type` is meta-type | `SyntaxRule` is meta-type for syntax |

### Comparison with Lisp/Macros

| Feature | Lisp Macros | This Experiment |
|---------|-------------|-----------------|
| Code Representation | S-expression (lists) | Type instances |
| Extension Method | defmacro | Define SyntaxRule type |
| Type Safety | Weak (text substitution) | Strong (type checking) |
| Parse Time | Runtime/Compile-time | Compile-time |
| IDE Support | Weak | Strong (type info) |

---

## Branch Plan

### Experimental Branch

```
Branch: exp/typed-homoiconicity
Created from: dev branch
```

**Important**:
- This is an **experimental branch**, won't merge with dev frequently
- May develop independently for a long time
- **No guarantee will merge to main**
- If experiment fails, branch will be abandoned

### Development Phases

> **⚠️ Experiment Time Limit: 6 months**

| Phase | Goal | Expected Time | Notes |
|-------|------|---------------|-------|
| Phase 1 | Proof of concept: Implement AST types with existing syntax | 2 weeks | |
| Phase 2 | Implement basic evaluator | 2 weeks | Key challenge: if/return control flow |
| Phase 3 | Implement SyntaxRule type parsing rules | 3 weeks | |
| Phase 4 | User-defined syntax extensions | 3 weeks | Core goal: at least one custom keyword runs |
| Phase 5 | Optimization and documentation | 2 weeks | Experiment ends |

**Timeout Handling**: If Phase 2 (control flow implementation) exceeds 4 weeks without progress, consider aborting.

---

## Trade-offs

### Advantages

- **Ultimate Unity**: Eliminate boundary between keywords and ordinary code
- **Language Extensibility**: Users can define their own syntax
- **Type Safety**: Safer than traditional macros
- **Learning Value**: Deeply understand language essence

### Disadvantages

- **Implementation Complexity**: Requires major compiler changes
- **Performance Concerns**: Runtime interpretation may be slow
- **Learning Curve**: Abstract concepts, requires understanding type system
- **Practicality Doubt**: May be over-engineered

### Risks

- Experiment may fail, cannot find practical use case
- Implementation difficulty exceeds expectations
- Conflicts with existing features

---

## Open Questions

- [ ] How to handle syntax conflicts (user-defined rules vs built-in)?
- [ ] Performance optimization plan?
- [ ] Need syntax import/export mechanism?
- [ ] How to integrate with existing module system?

---

## Appendix

### Glossary

| Term | Definition |
|------|------------|
| Homoiconicity | Code and data use the same representation |
| AST | Abstract Syntax Tree |
| SyntaxRule | Type carrying syntax parsing rules |
| Thunk | Function wrapper for delayed evaluation |
| CPS | Continuation Passing Style |

### References

- [Lisp Wiki: Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
- [Julia Metaprogramming](https://docs.julialang.org/en/v1/manual/metaprogramming/)
- [Rust Procedural Macros](https://doc.rust-lang.org/book/ch19-06-macros.html)

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
       ⚠️ Permanent experimental branch (exp/typed-homoiconicity)

       Possible outcomes:
       ├─► Success verification → Archive, NEVER merge
       ├─► Failure → Abandon branch
       └─► Timeout → Abort and abandon

       ⚠️ Regardless of outcome, this RFC NEVER merges
```

> **⚠️ Important Notice**: This is an exploratory experiment, **will NEVER merge**. Please do not rely on this feature in production code.
