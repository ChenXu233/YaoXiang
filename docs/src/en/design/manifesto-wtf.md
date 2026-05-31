# Critique of the YaoXiang Design Manifesto

> **Version**: v2.0.0 (After all, a "formally released" draft is still a release)  
> **Status**: Cerebral Orgasm  
> **Authors**: ChenXu + The "Community" That Has Yet to Assemble  
> **Date**: 2026-05-31 (From the future, but the compiler is still yesterday)

---

> 「The Tao gives birth to the One, the One gives birth to Two, Two gives birth to Three, and Three gives birth to the ten thousand things.」  
> —— *Tao Te Ching*  
>  
> **Types are like the Tao, all things are born from them.**  
> *(Programmers are like ants, all grinding away from it.)*

---

## I. Why Create YaoXiang? — Because the World Clearly Needs the 514th Language

### 1.1 The Language Gap We're Filling

In the long history of programming languages, we've witnessed countless languages born, become popular, and then tossed into history's trash bin. But **we are different** — we've keenly discovered a shocking gap: **竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适"。**

| Requirement | Problems with Existing Solutions | Our Solution (Projected) |
|-------------|----------------------------------|--------------------------|
| **Type Safety** | Rust is too strict, TypeScript is too loose | We'll create a quantum superposition type system that's both strict and loose |
| **Natural Syntax** | Everyone else's syntax is unnatural | Our syntax will be so natural you'll forget you're programming (or maybe you just won't understand it) |
| **AI Friendly** | AI-generated code often has errors | We're designing for AI comfort; humans can use it on the side |

### 1.2 Real Problems We're Solving

**Problem One: Fragmentation of Type Systems**  
We propose 「everything is a type」, solving the troubling philosophical question of "why aren't some things types?" Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem Two: The Binary Choice Between Memory Safety and Performance**  
We initially adopted Rust's ownership model, but discovered that the "borrow checker" was too difficult to implement. So we had a flash of inspiration — we renamed `&T` and `&mut T` from "references" to "tokens", declaring them "zero-sized compile-time permission proofs". Now we don't need a borrow checker anymore, just "flow-sensitive liveness analysis" — sounds completely different, right? If your program has a data race, it must be the token branding mechanism's fault.

**Problem Three: Cognitive Load of Async Programming**  
We reinvented the wheel and named it the "spawn model". Just one `spawn`, and the compiler handles all async details automatically — if it can't handle them, your code isn't "spawn" enough.

**Problem Four: Bottlenecks in AI-Assisted Programming**  
We've thoughtfully designed strict indentation and clear boundaries for AI, ensuring GPT-7 doesn't experience split personality when generating code. As for whether human programmers can understand it... that's secondary.

### 1.3 The Philosophical Roots of the Language

YaoXiang's name derives from the I Ching (Book of Changes), ensuring it comes with a mysticism buff in technical discussions. When your code fails to compile, you can say: "The yin and yang are not in harmony yet, let me divine a hexagram to take a look."

---

## II. Core Philosophy and Principles — Unquestionable Sacred Texts

### 2.1 Principle One: Everything Is a Type
**Non-negotiable reason**: This way we can explain everything with type theory, including why project schedules are always delayed.

### 2.2 Principle Two: Strictly Structured
**Non-negotiable reason**: 4-space indentation is cosmic truth. People who use Tab should be exiled to Mars.

### 2.3 Principle Three: Zero-Cost Abstractions
**Non-negotiable reason**: Although our abstraction layers number 7, since they're "zero-cost", performance should theoretically match hand-written assembly... in theory.

### 2.4 Principle Four: Immutable by Default
**Non-negotiable reason**: Mutability is the root of all evil. If you need to modify a variable, your design is wrong.

### 2.5 Principle Five: Types Are Data
**Non-negotiable reason**: This way we can check types at runtime, then discover... we already checked them at compile time.

---

## III. Key Innovations and Features — Reinventing Things Already Invented

### 3.1 Innovation One: Unified Type Syntax
We abolished the confusing concepts of `enum`, `struct`, `union`, `trait`, `impl`, and then abolished the `type` keyword itself. Now everything uses `name: Type = value`. Remember, `Type` is not a keyword — it's a reserved word. Don't ask what the difference is.

### 3.2 Innovation Two: Constructors Are Types
We eliminated the chasm between "types" and "values", creating a new chasm: "Is this a type constructor or a value constructor? Oh wait, they now use the same syntax, it's even harder to tell."

### 3.3 Innovation Three: Curried Method Binding
We achieved method calls through currying. Now you can use `Type.method = function[0]` instead of a `self` parameter. Clearly more intuitive. `[0]` means "treat the 0th parameter as self". If you forget to write `[0]`, the compiler will tell you "this is not a method, it's a regular function". Simple!

### 3.4 Innovation Four: Ownership Model (RFC-009 v9)
Five concepts, one gradient: `&T`, `&mut T`, Move, `ref`, `clone()`, `unsafe`. Wait, that's six. No matter — `&T` and `&mut T` are "tokens", not "references". The difference? References are a C++ concept; tokens are compile-time zero-sized type-level permission proofs. When your code fails to compile, you can say "type attribute Dup/Linear derivation failed", and no one will dare argue with you.

The token system also comes with these advanced features:

- **`freeze`**: "Freezes" `&mut T` into `&T`. Like putting fresh produce in the fridge — you can't cook until you thaw it. The compiler tracks freeze state through "flow-sensitive liveness analysis", which sounds like an ICU monitor.
- **Branding Mechanism**: Each token is assigned a unique compile-time integer (brand #N) for anti-counterfeiting. "I'm sorry sir, your `&Point` token's brand #42 doesn't match the brand #43 in the owner capsule."
- **Cannot Cross Tasks**: Tokens are "compile-time permission proofs" and cannot cross threads. If you need to share across tasks, use `ref`. Why? Because the compiler says so. Actually, it's because tokens disappear after compilation — zero-sized types, zero runtime overhead, zero cross-task capability.

Summary: Rust uses 200 pages of The Book to explain the borrow checker. YaoXiang explains everything with two sentences: "`&T` is copyable, `&mut T` is not copyable". Simplicity is beauty.

### 3.5 Innovation Five: The Spawn Model — The Worst Part of the Entire Language

> 「All things arise together, I observe their return.」 —— *I Ching · Hexagram Fu*

The spawn model's core selling point: **Synchronous syntax, asynchronous nature**. Translated into human language: Your code looks sequential, but the runtime automatically parallelizes it. When does it parallelize? How does it parallelize? The compiler decides. This isn't a concurrency model; it's a trust exercise.

Let's see what we stuffed into the language to achieve this magic:

**`spawn` keyword**: Marks a function as asynchronous. Note — not `async`, but `spawn`. Because `async` is too mainstream. But `spawn` in Rust means "start a task". No matter, we redefined it.

**`@block` annotation**: Marks a spawn function to "execute synchronously". Wait — if `spawn` is async, and `@block` makes it sync, why not just not write `spawn`? "Because sometimes you need a spawn function to run synchronously in certain contexts." So a function annotated with `spawn` may be async or sync depending on the caller's mood. This isn't a type system; it's split personality.

**`@eager` annotation**: Marks expressions that need "eager evaluation". Because the spawn model defaults to lazy evaluation — though lazy evaluation hasn't been implemented yet. So what's the actual effect of `@eager` right now? It's an IOU: "Someday, when lazy evaluation is implemented, this annotation will prevent this expression from being lazily evaluated."

**Summary of the three concurrency annotations**:
```
spawn  = this function will be async (unless @block'ed)
@block = this spawn function will be sync (overrides spawn)
@eager = this expression won't be lazily evaluated in the future (don't worry about it now)
```

If you find this confusing, congratulations — you understand. When your code parallelizes into a disaster, you can quote the I Ching to sound profound.

### 3.6 Innovation Six: Value-Dependent Types (RFC-011)
Now you can prove at compile time that your array length is prime, matrix dimensions must match, and the result of factorial(5) can be used in type signatures. Though this has nothing to do with writing business logic, "types are propositions, programs are proofs" — cool, right?

Don't forget the **decreases clause**: All recursively compile-time evaluated functions must prove they terminate. Otherwise your type checker enters an infinite loop, and your IDE becomes a space heater. "Sorry, your `factorial` function is missing a decreases clause. The compiler doesn't know if it will recurse to cosmic heat death when n=-1."

### 3.7 Innovation Seven: Minimalist Keyword Design
Only 17 keywords! Eight fewer than Go! Though each keyword's meaning is three times more complex than Go's keywords, we've won on quantity. Note: `type` is not a keyword — it was removed in RFC-010. Now you use `name: Type = value`, where `Type` is a reserved word. The difference between keywords and reserved words? Don't ask; asking leads to the compiler's internal universe-level Type0/Type1/Type2 issues.

### 3.8 Innovation Eight: Curry-Howard Isomorphism — The Universal Explaination

Whenever someone questions a design decision, the standard response is: "This follows the Curry-Howard isomorphism." Don't understand? Don't worry; no one in the community really does either. The gist is "types are propositions, programs are proofs", so your code isn't just a program — it's a mathematical paper. Compilation errors are proof by contradiction.

The crowning achievement of this philosophy is the easter egg in RFC-010: `Type: Type = Type`. Try compiling this line; the compiler won't crash — it will output a zen-like message along the lines of "The Tao that can be typed is not the eternal type; a type that can be type is not the eternal type". This is YaoXiang's tribute to Girard's paradox, and the only feature the compiler deliberately fails to implement. We call it "the language boundary" — when you reach it, the compiler falls silent here, and philosophy dwells.

---

## IV. Preliminary Syntax Preview — Code Examples That "Look Like They Work"

```yaoxiang
# === Hello World (runs in your head) ===
main: () -> Void = {
    print("Hello, future contributors!")
}

# === Ownership Model: Five Concepts (actually six) ===
Point: Type = { x: Float, y: Float }

p1 = Point(1.0, 2.0)
p2 = p1              # Move. Rest in peace, p1.
p2.print()           # Compiler creates &Point token. Token brand #4201, please collect.
p2.shift(1.0, 1.0)  # Compiler creates &mut Point token. Exclusive! Other tokens retreat!
shared = ref p2      # ref = sharing. Compiler automatically picks Rc. Or Arc. You don't need to know.
backup = p2.clone()  # Deep copy. Why not use ref? Because ref isn't copy, it's sharing. Get it?

# === Unified Syntax: name: type = value ===
# Can you tell which of the following is a type, which is a function, and which is a variable?
# Answer: No you can't. This is the beauty of "unification".
identity: (T: Type) -> ((x: T) -> T) = x
List: (T: Type) -> Type = { data: Array(T), length: Int }

# === Value-Dependent Types: Write factorial in the type signature ===
factorial: (n: Int) -> Int = {
    # decreases: n  ← Don't write this and the compiler panics
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
vec: Vec(factorial(5)) = Vec(120)()  # Vec(120) type, compile-time computation

# === Spawn Model: spawn + @block + @eager = Holy Trinity of Chaos ===
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

@block  # This line makes the spawn function above synchronous when called here
main: () -> Void = {
    data = fetch_data("https://api.example.com")  # Sync? Async? Depends on the mood.
}

# @eager: mark "don't lazily evaluate here when lazy evaluation is implemented"
result: Int eager = heavy_computation()  # Currently does exactly nothing

# Summary:
# spawn = async (unless @block)
# @block = makes spawn sync
# @eager = don't do something in the future (nothing happens now)
# Three concepts combined = if else
```

> *The above code runs beautifully in the documentation. Actual compilation results may vary. No, they will definitely vary.*

---

## V. Roadmap and Open Items — The Wish List

### 5.0 The RFC Dependency Triangle

Before understanding the roadmap, let's appreciate YaoXiang's most exquisite architectural design — the RFC love triangle:

```
RFC-009 (Ownership)  →  Depends on RFC-010 (Unified Syntax)  →  Depends on RFC-011 (Generics)
    ↑                                                              |
    └───────────────────── Dependency ────────────────────────────┘
```

009 needs 010's syntax, 010 needs 011's generics, 011 needs 009's type system. Three RFCs are mutually dependent. Which one do you implement first? "It is recommended to implement simultaneously." — RFC-010, line 141.

This is the Curry-Howard isomorphism in practical engineering: each RFC is a proposition, and their dependencies form a logical cycle. Breaking this cycle requires introducing an external axiom — i.e., "let's hardcode the type checker for now and deal with it later."

### 5.1 Already-Decided Design Decisions
**No longer accepting changes**, unless we change our minds.

### 5.2 Design Topics Under Discussion
Including "literal syntax", "generic inference", "pattern matching", and other trivial details. The core philosophy is already perfect; these minor matters can be discussed slowly.

### 5.3 Implementation Roadmap
```
v0.1: Rust interpreter ✅
v0.5: Bytecode compiler 🔄 (in progress, has been "in progress" for 18 months)
v1.0: Production ready   ⏳ (waiting to find the 10th contributor)
v2.0: Self-hosting       ⏳ (after we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status
- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse what should follow `spawn`)
- **Type Checker**: ✅ 95% (can determine `42` is type `Int`, but the type universe level of `Type` is still being debated)
- **Ownership Token System**: ✅ 100% (design document complete. Implementation? That's next.)
- **RFC Documents**: ✅ 14 accepted (average 800 lines each. Code? What code?)
- **Actually Runnable Code**: 🔴 0%

---

## VI. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

> *The authors listed in Cargo.toml: ["YaoXiang Team", "ChenXu2333"]. Team and ChenXu2333 are listed side by side. Upon investigation, Team's current size is 1 person. But this plural form "Team" leaves unlimited room for imagination.*

### 6.1 Design Discussions
**Suitable for**: People who enjoy theoretically debating "whether monads are monoids in the category of endofunctors".

### 6.2 Compiler Implementation
**Suitable for**: People with spare brain cells who don't mind using them to implement the 7th memory management model.

Most needed contributions currently:

- **Token Collision Detection**: Implement "flow-sensitive liveness analysis". Don't worry, even though the name is long, the principle is simple — track each token's state within the function body: live, frozen, moved. Like tracking three kids at a playground. Except the kids might infinitely recurse.
- **Cross-Task Cycle Detection Lint**: Detect `ref` cross-task circular references. Default warn, configurable to deny. We need someone to decide: how severe should the warn message be? "Warning: cross-task cycle detected" or "Warning: your code forms a cross-task cycle; while it won't leak, you should feel ashamed"?

### 6.3 Toolchain Development
**Tools to develop**: LSP server, debugger, formatter, package manager... **everything**. Especially the LSP — when the user hovers over `Type: Type = Type`, it should pop up a tooltip saying "the ineffable".

### 6.4 Standard Library Construction
From `std.io` to `std.gui`,应有尽有 (should have everything). What we currently have: `std.placeholder`. Next plan: `std.placeholder_v2`.

### 6.5 Documentation Translation
We need to translate the 14 RFCs into English. Each averages 800 lines. Approximately 11,200 lines total. Considering the RFCs are full of concepts like "spawn", "YaoXiang", "all things arise together, I observe their return", this is approximately equivalent to translating half of the Tao Te Ching. Sign up now.

### 6.7 Contribution Guidelines
**Commit message format**: Must be poetry. Sonnets preferred. Haiku also acceptable:
```
Ownership tokens
disappear after compilation
zero-cost abstraction
```

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**  
A: Less syntactic sugar! Fewer keywords! Fewer practical features! But more philosophical depth. Also, we have a "borrow token system" — sounds much more advanced than "borrow checker", right?

**Q: What types of development is YaoXiang suitable for?**  
A: Developing the YaoXiang compiler. And writing design manifestos and RFCs. Other use cases are under research.

**Q: Why choose 4-space indentation?**  
A: 2 spaces is too cramped, 8 spaces is too sparse; 4 spaces hits the golden mean, aligning with the spirit of the I Ching.

**Q: Is `Type` a keyword?**  
A: No. It's a "reserved word". The difference between keywords and reserved words is: keywords appear in the language specification's keyword list, reserved words appear in the "note: the following are not keywords" list. Simple and clear.

**Q: Why are there 14 accepted RFCs but the version is still 0.7.0?**  
A: Because we're playing a long game. Design first, implementation later. Really, really later.

**Q: Is `ref` actually Rc or Arc?**  
A: The compiler automatically chooses. You don't need to worry about it. Actually, this is the only time the compiler understands something better than the user, so we fully delegate.

**Q: When will the "spawn model" actually work?**  
A: When you read this line, the answer is still "design phase, not implemented". But the `spawn` keyword parses correctly, isn't that exciting?

**Q: When will version 1.0 be released?**  
A: When the "community" expands from 1 person to 2.

**Q: How do I contact the core team?**  
A: Leave a message on GitHub Discussions. Response time: 1-3 business months.

---

## VII. More Lies

**"Multi-language Support"**: `docs/src/{en,ja,ru,zh}` all four languages ready. Compiler v0.7.0, actually runnable lines of code approximately equal to zero, but Japanese and Russian developers can now read about "spawn model" and "value-dependent types" in their native language. This is classic "documentation-driven development" — let the whole world understand your design first, then pretend someone needs it. By the time the compiler can run Hello World, the documentation will have been translated into Klingon.

**Toolchain Matryoshka**: Python's pre-commit checks Rust's code style (cargo fmt + clippy), the Rust compiler compiles YaoXiang source. Three layers stacked, each depending on the next. When YaoXiang self-hosts, the matryoshka becomes: Python checks Rust, Rust compiles YaoXiang, YaoXiang compiles YaoXiang. By then, if one upstream dependency breaks, the entire toolchain becomes performance art. But that's fine — the word "bootstrapping" itself is worth two RFCs.

**YaoXiang-book.md**: A book systematically describing the YaoXiang language. Writing a book about a programming language not yet implemented is like publishing a travel guide for a non-existent city. "Chapter 3: Generic System — all code in this chapter cannot compile, but the syntax is correct. Please imagine the execution results." The most honest sentence in the entire book is on page one: "Project status: experimental verification phase".

**"No GC"**: Official stance: "YaoXiang has no GC." Strictly speaking, there's no tracing GC. But `ref` is reference-counted at runtime (Rc/Arc). Is reference counting GC? "No. GC is garbage collection, reference counting is automatic reference counting. You see, the abbreviations are different. One is GC, the other is ARC. Completely different." The significance of this wordplay: when someone says "isn't this just reference-counting GC?", you can say with righteous indignation "No, we have no GC, only compiler-managed reference counting". What's the difference? On the PPT.

> **Last Updated**: 2026-05-31 (possibly the last update, but you never know)  
>   
> **Document Version**: v2.0.0 (we jump version numbers quickly, to make it seem like we're making progress)  
>   
> **License**: MIT (it's the only MIT file we have anyway)

---

> 「YaoXiang changes, all things are born. Type evolves, programs are formed.」  
>   
> May your journey through YaoXiang's design become a **fascinating topic of conversation** over tea after meals.  
> *(After all, at this stage, it's mainly just a conversation topic.)*

---