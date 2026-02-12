# Critical Review of the YaoXiang Design Manifesto

> **Version**: v1.1.0 (after all, a "released" draft is still a release)
> **Status**: Intellectual euphoria
> **Authors**: ChenXu + the "community" that hasn't gathered yet
> **Date**: 2025-01-03 (from the future, but the syntax looks like it was written yesterday)

---

> "One generates two, two generate three, three generate all things."
> — Dao De Jing
>
> **Types are like the Dao, from which all things emerge.**
> *(Programmers are like ants, all suffering from this.)*

---

## 1. Why Create YaoXiang? — Because the World Clearly Needs the 514th Language

### 1.1 Bridging the Language Gap

Throughout the history of programming languages, we've witnessed countless languages born, become popular, and then thrown into the trash can of history. But **we're different**—we keenly discovered a shocking gap: **there somehow isn't a single language that can simultaneously make Rust enthusiasts find it too simple, make Python users find it too complex, and make AI models feel "comfortable" when generating code.**

| Need | Problems with Existing Solutions | Our Solution (Projected) |
|------|----------------------------------|--------------------------|
| **Type Safety** | Rust is too strict, TypeScript is too loose | We'll create a quantum superposition type system that's both strict and loose |
| **Natural Syntax** | Everyone else's syntax is unnatural | Our syntax will be so natural you'll forget you're programming (maybe because you can't understand it) |
| **AI-Friendly** | AI-generated code often has errors | We're designing syntax for AI, humans can use it incidentally |

### 1.2 Real Problems We Solve

**Problem One: Type System Fragmentation**
We propose "Everything is a Type," which solves the bothersome philosophical problem of "some things aren't types." Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem Two: The Choice Between Memory Safety and Performance**
We adopt Rust's ownership model, but removed those "annoying" compile errors. If your program has a data race, it must be the hardware's problem.

**Problem Three: Cognitive Burden of Async Programming**
We reinvented the wheel and named it "Concurrency Model." Just one `spawn`, and the compiler handles all async details—if it can't handle them, your code isn't "concurrent" enough.

**Problem Four: Bottlenecks in AI-Assisted Programming**
We thoughtfully designed strict indentation and clear boundaries for AI, ensuring GPT-7 won't split personalities when generating code. Whether human programmers can understand it... is secondary.

### 1.3 Philosophical Foundation of the Language

YaoXiang's name comes from the Book of Changes, which ensures it has a mysticism buff in technical discussions. When your code won't compile, you can say: "The yin and yang haven't balanced yet, let me consult the I Ching."

---

## 2. Core Philosophy and Principles — Unquestionable Holy Scripture

### 2.1 Principle One: Everything is a Type
**Non-negotiable reason**: This way we can explain everything with type theory, including why project schedules are always delayed.

### 2.2 Principle Two: Strictly Structured
**Non-negotiable reason**: 4-space indentation is the truth of the universe. People who use Tab should be exiled to Mars.

### 2.3 Principle Three: Zero-Cost Abstraction
**Non-negotiable reason**: Although our abstraction layer has 7 layers, since it's "zero-cost," performance should be about the same as hand-written assembly... in theory.

### 2.4 Principle Four: Immutable by Default
**Non-negotiable reason**: Mutability is the source of all evil. If you need to modify a variable, your design is wrong.

### 2.5 Principle Five: Type as Data
**Non-negotiable reason**: This way we can check types at runtime and discover... they were already checked at compile time.

---

## 3. Key Innovations and Features — Reinventing Things Already Invented

### 3.1 Innovation One: Unified Type Syntax
We abolished those confusing concepts of `enum`, `struct`, `union`, unified with `type`. Now you only need to remember 17 keywords, not 18. **Progress!**

### 3.2 Innovation Two: Constructors as Types
Eliminated the gap between "types" and "values," created a new gap: "Is this a type constructor or a value constructor?"

### 3.3 Innovation Three: Curried Method Binding
We achieved method calls through currying, so you can use `obj.method(_)` instead of `obj.method(_)`. Obviously more intuitive.

### 3.4 Innovation Four: Concurrency Model
> "All things grow together, and I observe their return." — I Ching, Hexagram Fu

When your code crashes in parallel, you can quote this and sound profound.

### 3.5 Innovation Five: Dependent Type Support
Now you can prove at compile time that your array length is prime. Although this has nothing to do with writing business logic, it's cool.

### 3.6 Innovation Six: Minimalist Keyword Design
Only 17 keywords! 8 fewer than Go! Although each keyword is 3 times more complex than Go keywords, we win on quantity.

---

## 4. Preliminary Syntax Preview — Code Examples That "Look Like They Work"

```yaoxiang
# This example runs in documentation
# In the actual compiler, it may need 3 more years to implement
main() -> Void = () => {
    println("Hello, future contributor!")
}
```

---

## 5. Roadmap and Pending Items — Dream List

### 5.1 Decided Design Decisions
**No longer accepting changes**, unless we change our minds.

### 5.2 Design Topics Under Discussion
Including "literal syntax," "generic inference," "pattern matching" and other trivial details. The core philosophy is already perfect, these small matters can be handled slowly.

### 5.3 Implementation Roadmap
```
v0.1: Interpreter Prototype ✅
v0.5: Compiler      🔄 (In progress, has been in progress for 6 months)
v1.0: Production Ready   ⏳ (Waiting for us to find the 10th contributor)
v2.0: Self-hosted       ⏳ (After we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status
- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse that there should be something after `spawn`)
- **Type Checker**: ✅ 95% (can determine that `42` is `Int` type)
- **Actually Runnable Code**: 🔴 0%

---

## 6. How to Contribute — Bring Your Time, Enthusiasm, and Lowered Expectations

### 6.1 Design Discussions
**Target Audience**: People who enjoy theoretically debating "whether a monad is a monoid in the category of endofunctors."

### 6.2 Compiler Implementation
**Target Audience**: Those with spare brain cells, and who don't mind them being used to implement the 7th memory management model.

### 6.3 Toolchain Development
**Tools to Develop**: LSP server, debugger, formatter, package manager... **everything**.

### 6.4 Standard Library Development
From `std.io` to `std.gui`,应有尽有。目前有的：`std.placeholder`. (Currently have: `std.placeholder`.)

### 6.7 Contribution Guide
**Commit Message Format**: Must be poetry. Sonnets preferred.

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have compared to Rust?**
A: Less syntactic sugar! Fewer keywords! Less practical functionality! But more philosophical depth.

**Q: What types of development is YaoXiang suitable for?**
A: Suitable for developing the YaoXiang compiler. Other uses pending research.

**Q: Why choose 4-space indentation?**
A: 2 spaces are too dense, 8 spaces are too sparse, 4 spaces are just right, conforming to the Doctrine of the Mean and the spirit of the Book of Changes.

**Q: When will version 1.0 be released?**
A: When the "community" expands from 1 person to 2 people.

**Q: How to contact the core team?**
A: Leave a message on GitHub Discussions. Response time: 1-3 business months.

---

> **Last Updated**: 2025-01-03 (possibly the last update)
>
> **Document Version**: v1.1.0 (our version numbers jump fast, making progress look faster)
>
> **License**: MIT (only MIT file exists anyway)

---

> "YaoXiang changes, all things are born. Types evolve, programs are formed."
>
> May the journey of YaoXiang's design become a **津津乐道的谈资** (frequently discussed topic) during your tea and rice breaks.
> *(After all, at this stage, it's mainly just a topic of conversation.)*
