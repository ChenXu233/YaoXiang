# A Critique of the YaoXiang Design Manifesto

> **Version**: v1.1.0 (after all, a "formally released" draft is still a release)
> **Status**: Orgasm of the mind
> **Author**: Morning Sunrise + the not-yet-gathered "community"
> **Date**: 2025-01-03 (from the future, but the syntax looks like it was written yesterday)

---

> 「The Tao gives birth to the One, the One gives birth to the Two, the Two gives birth to the Three, the Three gives birth to the ten thousand things.」
> —— *Tao Te Ching*
>
> **Types are like the Tao; all things are born from it.**
> *(Programmers are like ants, all crushed by it.)*

---

## I. Why Create YaoXiang? — Because the World Clearly Needs the 514th Language

### 1.1 The Language Gap We're Filling

In the long history of programming languages, we've witnessed countless languages being born, becoming popular, and then thrown into the garbage bin of history. But **we are different** — we've keenly discovered a shocking gap: **there isn't a single language that simultaneously makes Rust enthusiasts think it's too simple, Python users think it's too complex, and makes AI models feel "comfortable" when generating code**.

| Requirement | Problems with Existing Solutions | Our Solution (Estimated) |
|-------------|----------------------------------|--------------------------|
| **Type Safety** | Rust is too strict, TypeScript is too loose | We'll create a quantum superposition type system that is both strict and loose |
| **Natural Syntax** | Everyone else's syntax is unnatural | Our syntax will be so natural you'll forget you're programming (or maybe you just can't understand it) |
| **AI-Friendly** | AI-generated code often has errors | We're designing for AI; humans can use it on the side |

### 1.2 Real Problems We're Solving

**Problem One: Fragmentation of Type Systems**
We propose "everything is a type," solving the troubling philosophical problem of "some things not being types." Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem Two: The Binary Choice Between Memory Safety and Performance**
We adopt Rust's ownership model, but remove those "annoying" compile errors. If the program has data races, that's definitely a hardware problem.

**Problem Three: Cognitive Load of Async Programming**
We reinvented the wheel and named it the "Spawn Model." Simply use `spawn`, and the compiler handles all async details automatically — if it can't handle it, your code isn't "spawny" enough.

**Problem Four: Bottlenecks in AI-Assisted Programming**
We thoughtfully designed strict indentation and clear boundaries for AI, ensuring GPT-7 doesn't split personalities when generating code. As for whether human programmers can understand it... that's secondary.

### 1.3 Philosophical Roots of the Language

YaoXiang's name derives from the *I Ching* (Book of Changes), ensuring it comes with a mysticism buff in technical discussions. When your code won't compile, you can say: "The yin and yang are unbalanced; let me cast a hexagram to take a look."

---

## II. Core Philosophy and Principles — Unquestionable Holy Scripture

### Principle 1: Everything Is a Type
**Non-negotiable reason**: This way we can use type theory to explain everything, including why project progress is always delayed.

### Principle 2: Strictly Structured
**Non-negotiable reason**: 4-space indentation is cosmic truth. People who use Tab should be exiled to Mars.

### Principle 3: Zero-Cost Abstraction
**Non-negotiable reason**: Although our abstraction layer has 7 levels, since it's "zero-cost," performance should be roughly equivalent to hand-written assembly... in theory.

### Principle 4: Immutable by Default
**Non-negotiable reason**: Mutability is the root of all evil. If you need to modify a variable, it means your design is wrong.

### Principle 5: Types Are Data
**Non-negotiable reason**: This way we can check types at runtime, only to discover... we already checked them at compile time.

---

## III. Key Innovations and Features — Reinventing Already-Invented Things

### Innovation 1: Unified Type Syntax
We've abolished those confusing concepts of `enum`, `struct`, `union`, unifying them under `type`. Now you only need to remember 17 keywords instead of 18. **Progress!**

### Innovation 2: Constructors Are Types
We've eliminated the chasm between "type" and "value," creating a new chasm: "Is this a type constructor or a value constructor?"

### Innovation 3: Curried Method Binding
We've achieved method calls through currying, so you can use `obj.method(_)` instead of `obj.method()`. Clearly more intuitive.

### Innovation 4: Spawn Model

> 「All things arise together; I observe their return.」—— *I Ching, Hexagram 24 (Fu)*

When your code crashes in parallel, you can quote this, sounding very profound.

### Innovation 5: Dependent Type Support
Now you can prove at compile time that your array length is a prime number. While this has nothing to do with writing business logic, it's very cool.

### Innovation 6: Minimalist Keyword Design
Only 17 keywords! That's 8 fewer than Go! Although each keyword is 3 times more complex than a Go keyword in meaning, we win on quantity.

---

## IV. Preliminary Syntax Preview — Code Examples That "Look Like They Could Work"

```yaoxiang
# This example runs in the documentation
# In the actual compiler, it may take 3 more years to implement
main() -> Void = () => {
    println("Hello, future contributor!")
}
```

---

## V. Roadmap and Pending Items — The Dream List

### 5.1 Already-Decided Design Decisions
**No changes will be accepted**, unless we change our minds.

### 5.2 Design Topics Under Discussion
Including "literal syntax," "generic type inference," "pattern matching," and other trivial details. The core philosophy is already perfect; these small matters can be worked out gradually.

### 5.3 Implementation Roadmap

```
v0.1: Interpreter prototype ✅
v0.5: Compiler      🔄 (in progress, has been in progress for 6 months)
v1.0: Production-ready   ⏳ (waiting until we find the 10th contributor)
v2.0: Self-hosting       ⏳ (after we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status

- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse that there should be something after `spawn`)
- **Type Checker**: ✅ 95% (can determine that `42` is of type `Int`)
- **Actually runnable code**: 🔴 0%

---

## VI. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

### 6.1 Design Discussions
**Suitable for**: People who enjoy theoretically debating "whether monads are monoids in the category of endofunctors."

### 6.2 Compiler Implementation
**Suitable for**: People with spare brain cells who don't mind them being used to implement the 7th memory management model.

### 6.3 Toolchain Development
**Tools needing development**: LSP server, debugger, formatter, package manager... **everything**.

### 6.4 Standard Library Construction
From `std.io` to `std.gui`, it should have everything. What we currently have: `std.placeholder`.

### 6.7 Contribution Guidelines
**Commit message format**: Must be poetry. Sonnets preferred.

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**
A: Less syntactic sugar! Fewer keywords! Fewer practical features! But more philosophical depth.

**Q: What types of development is YaoXiang suitable for?**
A: Suitable for developing the YaoXiang compiler. Other use cases are under research.

**Q: Why choose 4-space indentation?**
A: 2 spaces are too cramped, 8 spaces are too spread out; 4 spaces are just right for the Doctrine of the Mean, aligning with the spirit of the *I Ching*.

**Q: When will version 1.0 be released?**
A: When the "community" expands from 1 person to 2 people.

**Q: How do I contact the core team?**
A: Leave a message on GitHub Discussions. Response time: 1-3 business months.

---

> **Last Updated**: 2025-01-03 (possibly the last update)
>
> **Document Version**: v1.1.0 (we bump version numbers quickly; it makes progress look faster)
>
> **License**: MIT (what else would it be at this stage)

---

> 「The changes of YaoXiang give birth to all things. The evolution of types creates programs.」
>
> May YaoXiang's design journey become **a conversation piece for your tea breaks**.
> *(After all, at this stage, it's mainly just a conversation piece.)*