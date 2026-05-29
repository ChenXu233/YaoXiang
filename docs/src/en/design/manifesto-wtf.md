# A Sharp Critique of the YaoXiang Design Manifesto

> **Version**: v1.1.0 (After all, a "formally released" draft is still a release)  
> **Status**: Cerebral Orgasm  
> **Authors**: Chen Xu + the "Community" Yet to Gather  
> **Date**: 2025-01-03 (From the future, but the syntax looks like it was written yesterday)

---

> "The Tao gives birth to the One, the One gives birth to Two, Two gives birth to Three, and Three gives birth to all things."  
> — *Dao De Jing*  
>  
> **Types are like the Tao, all things are born from them.**  
> *(Programmers are like ants, all suffering from it.)*

---

## I. Why Create YaoXiang? — Because the World Clearly Needs the 514th Programming Language

### 1.1 The Language Gap We're Filling

In the long history of programming languages, we've witnessed countless languages born, become popular, and then thrown into the dustbin of history. But **we are different** — we keenly discovered a shocking gap: **竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适"**。

| Demand | Problems with Existing Solutions | Our Solution (Projected) |
|--------|-----------------------------------|---------------------------|
| **Type Safety** | Rust is too strict, TypeScript is too loose | We'll create a quantum superposition type system that's both strict and loose |
| **Natural Syntax** | Everyone else's syntax is unnatural | Our syntax will be so natural you'll forget you're programming (or maybe you just can't read it) |
| **AI-Friendly** | AI-generated code often has errors | We're designing syntax for AI; humans can use it on the side |

### 1.2 Real Problems We're Solving

**Problem One: Fragmentation of the type system**  
We propose "everything is a type," which solves the troubling philosophical problem of "some things aren't types." Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem Two: The binary choice between memory safety and performance**  
We adopt Rust's ownership model, but remove those "annoying" compile errors. If there's a data race, it must be the hardware's problem.

**Problem Three: The cognitive burden of asynchronous programming**  
We've reinvented the wheel and named it the "spawn model." Just one `spawn`, and the compiler handles all async details automatically — if it can't, your code isn't "spawning" correctly.

**Problem Four: Bottlenecks in AI-assisted programming**  
We've thoughtfully designed strict indentation and clear boundaries for AI, ensuring GPT-7 doesn't have an identity crisis when generating code. Whether human programmers can understand it... is secondary.

### 1.3 The Philosophical Roots of the Language

YaoXiang's name comes from the *Yijing* (I Ching), which ensures it has a built-in mysticism buff in technical discussions. When your code won't compile, you can say: "The yin and yang haven't harmonized yet; let me divine a hexagram."

---

## II. Core Philosophy and Principles — Unquestionable Dogma

### 2.1 Principle One: Everything is a Type
**Non-negotiable reason**: This way we can use type theory to explain everything, including why project progress is always delayed.

### 2.2 Principle Two: Strict Structure
**Non-negotiable reason**: 4-space indentation is universal truth. People who use Tab should be exiled to Mars.

### 2.3 Principle Three: Zero-Cost Abstraction
**Non-negotiable reason**: Although our abstraction layer has 7 levels, since it's "zero-cost," performance should be comparable to hand-written assembly... in theory.

### 2.4 Principle Four: Immutability by Default
**Non-negotiable reason**: Mutability is the root of all evil. If you need to modify a variable, it means your design was wrong.

### 2.5 Principle Five: Types as Data
**Non-negotiable reason**: This way we can check types at runtime, only to discover... they were already checked at compile-time.

---

## III. Key Innovations and Features — Reinventing Things Already Invented

### 3.1 Innovation One: Unified Type Syntax
We've abolished those confusing concepts of `enum`, `struct`, `union`, and unified them under `type`. Now you only need to remember 17 keywords instead of 18. **Progress!**

### 3.2 Innovation Two: Constructors as Types
We've bridged the chasm between "type" and "value," creating a new chasm: "Is this a type constructor or a value constructor?"

### 3.3 Innovation Three: Curried Method Binding
We've achieved method invocation through currying, so you can use `obj.method(_)` instead of `obj.method()`. Clearly more intuitive.

### 3.4 Innovation Four: The Spawn Model
> "All things rise and act together; I observe their return." — *Yijing, Hexagram Fu*

When your code crashes in parallel, you can quote this to sound profound.

### 3.5 Innovation Five: Dependent Type Support
Now you can prove at compile-time that your array length is a prime number. While this has nothing to do with writing business logic, it's very cool.

### 3.6 Innovation Six: Minimalist Keyword Design
Only 17 keywords! That's 8 fewer than Go! Although each keyword is 3 times more complex than Go's keywords in meaning, we've won on quantity.

---

## IV. Preliminary Syntax Preview — Code Examples That "Look Like They Could Work"

```yaoxiang
# This example runs in documentation
# In the actual compiler, it may take 3 more years to implement
main() -> Void = () => {
    println("Hello, future contributor!")
}
```

---

## V. Roadmap and Pending Items — The Dream List

### 5.1 Already-Decided Design Decisions
**No changes accepted**, unless we change our minds.

### 5.2 Design Topics Under Discussion
Including "literal syntax," "generic inference," "pattern matching," and other trivial details. The core philosophy is already perfect; these small matters can be worked out gradually.

### 5.3 Implementation Roadmap
```
v0.1: Interpreter prototype ✅
v0.5: Compiler      🔄 (in progress, 6 months and counting)
v1.0: Production-ready   ⏳ (waiting to find our 10th contributor)
v2.0: Self-hosting       ⏳ (after we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status
- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse what should follow `spawn`)
- **Type checker**: ✅ 95% (can determine `42` is of type `Int`)
- **Actually runnable code**: 🔴 0%

---

## VI. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

### 6.1 Design Discussions
**Suitable for**: People who enjoy theoretically debating "whether monads are monoids in the category of endofunctors."

### 6.2 Compiler Implementation
**Suitable for**: Those with spare brain cells, and who don't mind them being used to implement the 7th memory management model.

### 6.3 Toolchain Development
**Tools to develop**: LSP server, debugger, formatter, package manager... **everything**.

### 6.4 Standard Library Development
From `std.io` to `std.gui`, we have it all. Currently available: `std.placeholder`.

### 6.7 Contribution Guidelines
**Commit message format**: Must be poetry. Sonnets preferred.

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**  
A: Less syntax sugar! Fewer keywords! Fewer practical features! But more philosophical depth.

**Q: What types of development is YaoXiang suitable for?**  
A: Suitable for developing the YaoXiang compiler. Other uses are under research.

**Q: Why choose 4-space indentation?**  
A: 2 spaces is too cramped, 8 spaces is too sparse, 4 spaces hits the golden mean, aligning with the spirit of the *Yijing*.

**Q: When will version 1.0 be released?**  
A: When the "community" expands from 1 person to 2.

**Q: How do I contact the core team?**  
A: Leave a message in GitHub Discussions. Response time: 1-3 business months.

---

> **Last updated**: 2025-01-03 (possibly the last update)  
>   
> **Document version**: v1.1.0 (we version-bump quickly, to make progress look faster)  
>   
> **License**: MIT (we only have an MIT file anyway)

---

> "YaoXiang changes, all things are born. Type evolves, programs are formed."  
>   
> May YaoXiang's design journey become **a conversation piece for your tea and after-dinner chats**.  
> *(After all, at this stage, it's mainly a conversation piece.)*

---