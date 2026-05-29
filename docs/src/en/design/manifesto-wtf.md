# "YaoXiang Design Manifesto" Critique

> **Version**: v1.1.0 (After all, a "formally released" draft is still a release)
> **Status**: Cerebral Orgasm
> **Authors**: Chen Xu + the "Community" Yet to Gather
> **Date**: 2025-01-03 (From the future, but the syntax looks like it was written yesterday)

---

> 「The Tao gave birth to the One, the One gave birth to the Two, the Two gave birth to the Three, and the Three gave birth to all things.」
> —— *Dao De Jing*
>
> **Types are like the Tao, all things are born from them.**
> *(Programmers are like ants, all exhausted by it.)*

---

## I. Why Create YaoXiang? — Because the World Clearly Needs the 514th Language

### 1.1 The Language Gap We're Filling

In the long river of programming language history, we've witnessed countless languages born, become popular, then thrown into the dustbin of history. But **we are different** — we've keenly discovered a staggering gap: **竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适"** — *，竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适"* — there is，竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适"，竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适".

| Need | Problems with Existing Solutions | Our Solution (Projected) |
|------|----------------------------------|--------------------------|
| **Type safety** | Rust is too strict, TypeScript is too loose | We will create a quantum superposition type system that is both strict and loose |
| **Natural syntax** | Everyone else's syntax is unnatural | Our syntax will be so natural you'll forget you're programming (or maybe it's because you can't understand it) |
| **AI-friendly** | AI-generated code often has errors | We're designing syntax for AI; humans can use it on the side |

### 1.2 Practical Problems We're Solving

**Problem 1: Fragmentation of the type system**
We propose "everything is a type," which solves the troubling philosophical problem of "some things not being types." Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem 2: The false dichotomy of memory safety vs. performance**
We adopt Rust's ownership model but remove those "annoying" compile errors. If there's a data race, it must be the hardware's problem.

**Problem 3: The cognitive burden of async programming**
We reinvented the wheel and named it the "spawn model." With just one `spawn`, the compiler handles all async details automatically — if it can't handle it, your code isn't "spawny" enough.

**Problem 4: Bottlenecks in AI-assisted programming**
We've thoughtfully designed strict indentation and clear boundaries for AI, ensuring GPT-7 won't have an identity crisis when generating code. As for whether human programmers can understand it... that's secondary.

### 1.3 The Philosophical Roots of the Language

YaoXiang's name derives from the *I Ching*, ensuring it comes with a mysticism buff in technical discussions. When your code won't compile, you can say: "The yin and yang are unbalanced; let me consult the hexagrams."

---

## II. Core Philosophy and Principles — Unquestionable Sacred Texts

### 2.1 Principle 1: Everything is a Type
**Non-negotiable reason**: This way we can explain everything with type theory, including why project deadlines are always delayed.

### 2.2 Principle 2: Strictly Structured
**Non-negotiable reason**: 4-space indentation is the absolute truth. Tab users should be exiled to Mars.

### 2.3 Principle 3: Zero-Cost Abstraction
**Non-negotiable reason**: Although we have 7 layers of abstraction, since they're "zero-cost," performance should be roughly equivalent to hand-written assembly... in theory.

### 2.4 Principle 4: Immutable by Default
**Non-negotiable reason**: Mutability is the root of all evil. If you need to modify a variable, it means your design is wrong.

### 2.5 Principle 5: Types are Data
**Non-negotiable reason**: This way we can check types at runtime, only to discover... they were already checked at compile-time.

---

## III. Key Innovations and Features — Reinventing Already-Invented Things

### 3.1 Innovation 1: Unified Type Syntax
We've abolished confusing concepts like `enum`, `struct`, and `union`, unifying them under `type`. Now you only need to remember 17 keywords instead of 18. **Progress!**

### 3.2 Innovation 2: Constructors are Types
We've eliminated the chasm between "type" and "value," creating a new chasm: "Is this a type constructor or a value constructor?"

### 3.3 Innovation 3: Curried Method Binding
We've achieved method calls through currying, so you can use `obj.method(_)` instead of `obj.method()`. Obviously more intuitive.

### 3.4 Innovation 4: Spawn Model
> 「All things rise and fall; I observe their return.」—— *Hexagram Fu (Recovery)*

When your code crashes in parallel, you can quote this to sound profound.

### 3.5 Innovation 5: Dependent Type Support
Now you can prove at compile-time that your array length is prime. Though this has nothing to do with writing business logic, it's pretty cool.

### 3.6 Innovation 6: Minimalist Keyword Design
Only 17 keywords! That's 8 fewer than Go! Although each keyword is 3 times more complex than a Go keyword, we win on quantity.

---

## IV. Preliminary Syntax Preview — Code Examples that "Look Like They Work"

```yaoxiang
# This example runs in the documentation
# In the actual compiler, it may take another 3 years to implement
main() -> Void = () => {
    println("Hello, future contributor!")
}
```

---

## V. Roadmap and Open Items — The Dream List

### 5.1 Already-Decided Design Decisions
**No longer accepting changes**, unless we change our minds.

### 5.2 Design Topics Under Discussion
Including "literal syntax," "generic inference," "pattern matching," and other trivial details. The core philosophy is already perfect; these small matters can be sorted out later.

### 5.3 Implementation Roadmap

```
v0.1: Interpreter prototype     ✅
v0.5: Compiler                  🔄 (in progress, has been in progress for 6 months)
v1.0: Production-ready          ⏳ (waiting until we find the 10th contributor)
v2.0: Self-hosting              ⏳ (after we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status

- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse what should come after `spawn`)
- **Type checker**: ✅ 95% (can determine that `42` is of type `Int`)
- **Actually runnable code**: 🔴 0%

---

## VI. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

### 6.1 Design Discussions
**Suitable for**: People who enjoy theoretically debating "whether a monad is a monoid in the category of endofunctors."

### 6.2 Compiler Implementation
**Suitable for**: People with spare brain cells who don't mind using them to implement the 7th memory management model.

### 6.3 Tooling Development
**Tools needing development**: LSP servers, debuggers, formatters, package managers... **everything**.

### 6.4 Standard Library Construction
From `std.io` to `std.gui`, we have it all. Currently available: `std.placeholder`.

### 6.7 Contribution Guidelines
**Commit message format**: Must be poetry. Sonnets preferred.

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**
A: Less syntactic sugar! Fewer keywords! Fewer practical features! But more philosophical depth.

**Q: What types of development is YaoXiang suitable for?**
A: Suitable for developing the YaoXiang compiler. Other use cases are under research.

**Q: Why choose 4-space indentation?**
A: 2 spaces are too cramped, 8 spaces are too spread out, 4 spaces hit the golden mean, aligning with the spirit of the *I Ching*.

**Q: When will version 1.0 be released?**
A: When the "community" expands from 1 person to 2.

**Q: How do I reach the core team?**
A: Leave a message on GitHub Discussions. Response time: 1-3 business months.

---

> **Last updated**: 2025-01-03 (possibly the last update)
>
> **Document version**: v1.1.0 (we version-bump quickly, to make progress look faster)
>
> **License**: MIT (we only have MIT files anyway)

---

> 「YaoXiang changes, all things are born. Types evolve, programs are formed.」
>
> May YaoXiang's design journey become a **topic of lively conversation over tea and after dinner** for you.
> *(After all, at this stage, it's mainly just a conversation piece.)*