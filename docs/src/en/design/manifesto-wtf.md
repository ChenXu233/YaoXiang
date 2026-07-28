# YaoXiang Design Manifesto - A Critical Review

> **Version**: v2.0.0 (After all, a "formally released" draft is still a release)
> **Status**: Intracranial Orgasm
> **Author**: ChenXu + The Not-Yet-Gathered "Community"
> **Date**: 2026-05-31 (From the future, but the compiler is still yesterday)

---

> 「The Tao gives birth to the One, the One gives birth to the Two, the Two gives birth to the Three, and the Three gives birth to all things.」
> —— *Dao De Jing*
>
> **Types are like the Tao, from which all things are born.**
> *（Programmers are like ants, all suffering from it.）*

---

## 1. Why Create YaoXiang? — Because the World Clearly Needs the 514th Language

### 1.1 The Language Gap We're Filling

In the long history of programming languages, we've witnessed countless languages born, become popular, and then thrown into the dustbin of history. But **we are different** — we've keenly discovered a shocking gap: **竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适"**。*There is，竟然没有一门语言能同时让 Rust 爱好者觉得太简单、让 Python 用户觉得太复杂、并且让 AI 模型在生成代码时感到"舒适"：no language that simultaneously makes Rust enthusiasts think it's too simple, Python users think it's too complex, and AI models feel "comfortable" when generating code.*

| Requirement     | Problems with Existing Solutions              | Our Solution (Projected)                                                    |
| --------------- | --------------------------------------------- | --------------------------------------------------------------------------- |
| **Type Safety** | Rust is too strict, TypeScript is too loose   | We will create a quantum superposition type system that is both strict and loose |
| **Natural Syntax** | Other people's syntax is all unnatural       | Our syntax will be so natural you'll forget you're programming (or maybe it's because you can't understand it) |
| **AI Friendly** | AI-generated code often has errors           | We're designing syntax for AI; humans can use it incidentally                 |

### 1.2 Practical Problems We're Solving

**Problem One: Type System Fragmentation**  
We propose「Everything is a Type」, which solves the troubling philosophical question of "why aren't some things types." Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem Two: The Binary Choice Between Memory Safety and Performance**  
We initially adopted Rust's ownership model, but found the "borrow checker" too difficult to implement. So we had a flash of inspiration — we renamed `&T` and `&mut T` from "references" to "tokens", declaring them "zero-sized compile-time permission proofs." Now we don't need a borrow checker, just "flow-sensitive liveness analysis" — sounds completely different, right? If the program has a data race, it must be the token branding mechanism's fault.

**Problem Three: Cognitive Load of Async Programming**  
We reinvented the wheel and named it the "spawn model." With just one `spawn`, the compiler automatically handles all async details — if it can't handle them, your code isn't "spawny" enough.

**Problem Four: Bottlenecks in AI-Assisted Programming**  
We thoughtfully designed strict indentation and clear boundaries for AI, ensuring GPT-7 doesn't split personalities when generating code. As for whether human programmers can understand it... that's secondary.

### 1.3 Philosophical Roots of the Language

YaoXiang's name comes from the *Yijing* (I Ching), which ensures it comes with a mysticism buff in technical discussions. When your code won't compile, you can say: "The yin and yang are not yet in harmony; let me divine to check."

---

## 2. Core Philosophy and Principles — Unquestionable Sacred Texts

### 2.1 Principle One: Everything is a Type

**Non-negotiable reason**: This allows us to explain everything with type theory, including why project schedules are always delayed.

### 2.2 Principle Two: Strictly Structured

**Non-negotiable reason**: 4-space indentation is the absolute truth. People who use Tab should be exiled to Mars.

### 2.3 Principle Three: Zero-Cost Abstraction

**Non-negotiable reason**: Although our abstraction has 7 layers, since it's "zero-cost," performance should be roughly equivalent to hand-written assembly... theoretically.

### 2.4 Principle Four: Immutable by Default

**Non-negotiable reason**: Mutability is the root of all evil. If you need to modify a variable, it means your design is wrong.

### 2.5 Principle Five: Types are Data

**Non-negotiable reason**: This way we can check types at runtime, then discover... that they were already checked at compile time.

---

## 3. Key Innovations and Features — Reinventing Things Already Invented

### 3.1 Innovation One: Unified Type Syntax

We abolished confusing concepts like `enum`, `struct`, `union`, `trait`, `impl`, and then abolished the `type` keyword itself. Now everything uses `name: Type = value`. Remember, `Type` is not a keyword — it's a reserved word; don't ask what the difference is.

### 3.2 Innovation Two: Constructors are Types

We've eliminated the chasm between "types" and "values," creating a new chasm: "Is this a type constructor or a value constructor? Oh wait, they use the same syntax now, so it's even harder to tell."

### 3.3 Innovation Three: Curried Method Binding

We've implemented method calls through currying. Now you can use `Type.method = function[0]` instead of `self` parameters. Obviously more intuitive. `[0]` means "treat the 0th parameter as self"; if you forget to write `[0]`, the compiler will tell you "this is not a method, it's a regular function." Simple!

### 3.4 Innovation Four: Ownership Model (RFC-009 v9)

Five concepts, one gradient: `&T`, `&mut T`, Move, `ref`, `clone()`, `unsafe`. Wait, that's six. No matter — `&T` and `&mut T` are "tokens," not "references." The difference? References are a C++ concept; tokens are compile-time zero-sized type-level permission proofs. When your code won't compile, you can say "Type attribute Dup/Linear derivation failed," and no one will dare argue with you.

The token system also comes with these advanced features:

- **`freeze`**: "Freeze" a `&mut T` into `&T`. Like putting fresh produce in the fridge — you can't cook it until you thaw it. The compiler uses "flow-sensitive liveness analysis" to track freeze states, which sounds like an ICU monitor.
- **Branding Mechanism**: Each token is assigned a unique integer at compile time (brand #N) for anti-counterfeiting. "I'm sorry sir, your `&Point` token's brand #42 doesn't match the brand #43 in your owner capsule."
- **Cannot Cross Tasks**: Tokens are "compile-time permission proofs" and cannot cross threads. If you need to share across tasks, use `ref`. Why? Because the compiler says so. Actually, it's because tokens disappear after compilation — zero-sized types, zero runtime overhead, and zero cross-task capability.

Summary: Rust explains the borrow checker in 200 pages of The Book. YaoXiang explains everything in two sentences: "`&T` is copyable, `&mut T` is not copyable." Simplicity is beauty.

### 3.5 Innovation Five: The Spawn Model — The Worst Part of the Entire Language

> 「All things arise together; I observe their return.」—— *Yijing, Hexagram 24*

The spawn model's core selling point: **Synchronous syntax, asynchronous nature**. In human terms: your code looks like it's executing sequentially, but the runtime will automatically parallelize it. When does it parallelize? How does it parallelize? The compiler decides. This isn't a concurrency model; it's a game of trust.

Here's what we've stuffed into the language to achieve this magic:

**`spawn` keyword**: Marks a function as asynchronous. Note — not `async`, it's `spawn`. Because `async` is too mainstream. But `spawn` in Rust means "start a task." No matter, we redefined it.

**`@block` annotation**: Marks a spawn function that should "execute synchronously." Wait — if `spawn` is async, and `@block` makes it sync, why not just not write `spawn`? "Because sometimes you need a spawn function to run synchronously in certain contexts." So a function marked `spawn` might be async or sync depending on the caller's mood. This isn't a type system; it's split personality.

**`@eager` annotation**: Marks expressions that need "eager evaluation." Because the spawn model uses lazy evaluation by default — though lazy evaluation hasn't been implemented yet. So what is `@eager`'s actual effect right now? It's an IOU: "Someday, when lazy evaluation is implemented, this annotation will prevent this expression from being lazily evaluated."

**Summary of the Three Annotations for Concurrency**:

```
spawn  = This function will be async (unless overridden by @block)
@block = This spawn function will be sync (overrides spawn)
@ager = This expression will not be lazily evaluated in the future (Don't worry about the future yet)
```

If you find this confusing, congratulations — you understand it. When your code crashes in parallel, you can quote the Yijing to sound profound.

### 3.6 Innovation Six: Value-Dependent Types (RFC-011)

Now you can prove at compile time that your array length is prime, matrix dimensions must match, and that the result of `factorial(5)` can be used in type signatures. Though this has nothing to do with writing business logic, "types are propositions, programs are proofs" — isn't that cool?

Don't forget the **decreases clause**: All recursively computed-at-compile-time functions must prove they'll terminate. Otherwise your type checker will spin into an infinite loop, and your IDE will become a space heater. "Sorry, your `factorial` function is missing a decreases clause. The compiler doesn't know if it'll recurse to cosmic heat death when n=-1."

### 3.7 Innovation Seven: Minimalist Keyword Design

Only 17 keywords! That's 8 fewer than Go! Though each keyword is 3 times more complex than a Go keyword, we win on quantity. Note: `type` is not a keyword — it was removed in RFC-010. Now you use `name: Type = value`, where `Type` is a reserved word. The difference between keywords and reserved words? Don't ask; if you ask, it's a question about the compiler's internal universe-level Type0/Type1/Type2 system.

### 3.8 Innovation Eight: Curry-Howard Isomorphism — The Universal Explanation

Whenever someone questions a design decision, the standard answer is: "This follows the Curry-Howard isomorphism." Don't understand? No problem; no one in the community really understands either. Roughly, it means "types are propositions, programs are proofs," so your code isn't just a program — it's a mathematical paper. Compile errors are proof by contradiction.

The crowning achievement of this philosophy is the Easter egg in RFC-010: `Type: Type = Type`. Try compiling this line; the compiler won't crash — it will output a zen message along the lines of "The Tao that can be typed is not the eternal Type; the type that can be type-checked is not the eternal Type." This is YaoXiang's tribute to Girard's paradox, and the only feature the compiler intentionally doesn't implement. We call it the "language boundary" — when you reach it, the compiler falls silent here, and philosophy dwells.

---

## 4. Preliminary Syntax Preview — Code Examples That "Look Like They Could Work"

```yaoxiang
# === Hello World (runs in your head) ===
main: () -> Void = {
    print("Hello, future contributor!")
}

# === Ownership Model: Five Concepts (actually six) ===
Point: Type = { x: Float, y: Float }

p1 = Point(1.0, 2.0)
p2 = p1              # Move. p1, rest in peace.
p2.print()           # Compiler creates &Point token. Token brand #4201, please verify.
p2.shift(1.0, 1.0)   # Compiler creates &mut Point token. Exclusive! Other tokens retreat!
shared = ref p2      # ref = shared. Compiler automatically chooses Rc. Or Arc. You don't need to know.
backup = p2.clone()  # Deep copy. Why not use ref? Because ref isn't copy, it's shared. Get it?

# === Unified Syntax: name: type = value ===
# Can you tell which below is a type, which is a function, which is a variable?
# Answer: No. This is the beauty of "unification."
identity: (T: Type) -> ((x: T) -> T) = x
List: (T: Type) -> Type = { data: Array(T), length: Int }

# === Value-Dependent Types: Write factorial in the type signature ===
factorial: (n: Int) -> Int = {
    # decreases: n  ← Compiler panics without this
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
vec: Vec(factorial(5)) = Vec(120)()  # Vec(120) type, computed at compile time

# === Spawn Model: spawn + @block + @eager = Holy Trinity of Chaos ===
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

@block  # This line makes the spawn function above synchronous when called here
main: () -> Void = {
    data = fetch_data("https://api.example.com")  # Sync? Async? Depends on the mood.
}

# @eager: Mark "don't lazily evaluate here when lazy evaluation is implemented"
result: Int eager = heavy_computation()  # Currently the same as not writing it

# Summary:
# spawn = async (unless @block)
# @block = make spawn sync
# @eager = don't do something in the future (nothing happens now)
# Three concepts combined = if else
```

> *The above code runs beautifully in documentation. Actual compilation results may vary. No, they definitely will vary.*

---

## 5. Roadmap and Pending Items — The Dream List

### 5.0 The RFC Dependency Triangle

Before understanding the roadmap, let's admire YaoXiang's most exquisite architectural design — the RFC love triangle:

```
RFC-009 (Ownership)  →  Depends on RFC-010 (Unified Syntax)  →  Depends on RFC-011 (Generics)
    ↑                                                                         │
    └───────────────────── Depends ──────────────────────────────────────────┘
```

009 needs 010's syntax, 010 needs 011's generics, and 011 needs 009's type system. Three RFCs are mutually dependent. Which one to implement first? "Recommended to implement simultaneously." — RFC-010, line 141.

This is the Curry-Howard isomorphism in practical engineering: each RFC is a proposition, and their dependencies form a logical cycle. Breaking this cycle requires introducing an external axiom — namely, "let's hardcode the type checker first and figure it out later."

### 5.1 Design Decisions That Are Already Made

**No more changes accepted**, unless we change our minds.

### 5.2 Design Topics Still Under Discussion

Including "literal syntax," "generic inference," "pattern matching," and other trivial details. The core philosophy is already perfect; these minor matters can be discussed slowly.

### 5.3 Implementation Roadmap

```
v0.1: Rust interpreter ✅
v0.5: Bytecode compiler 🔄 (In progress, has been in progress for 18 months)
v1.0: Production ready   ⏳ (Waiting for us to find the 10th contributor)
v2.0: Self-hosting       ⏳ (After we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status

- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse what should follow `spawn`)
- **Type Checker**: ✅ 95% (can determine `42` is of type `Int`, but the universe level of `Type` is still being debated)
- **Ownership Token System**: ✅ 100% (design document complete. Implementation? That's the next step.)
- **RFC Documents**: ✅ 14 accepted (average 800 lines each. Code? What code?)
- **Actually Runnable Code**: 🔴 0%

---

## 6. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

> *The authors listed in Cargo.toml: ["YaoXiang Team", "ChenXu2333"]. Team and ChenXu2333 are listed side by side. Upon investigation, Team's current size is 1 person. But this plural form "Team" gives infinite room for imagination.*

### 6.1 Design Discussions

**Suitable for**: People who enjoy theoretically debating "whether monads are monoids in the category of endofunctors."

### 6.2 Compiler Implementation

**Suitable for**: People with idle brain cells who don't mind them being used to implement the 7th memory management model.

Currently most needed contributions:

- **Token Conflict Detection**: Implement "flow-sensitive liveness analysis." Don't worry, despite the long name, the principle is simple — just track each token's state in the function body: live, frozen, moved. Like tracking three kids at a playground. Except the kids might infinitely recurse.
- **Cross-Task Cycle Detection Lint**: Detect cross-task circular references in `ref`. Default warn, configurable deny. We need someone to decide: how stern should the warn wording be? "Warning: cross-task cycle detected" or "Warning: your code has formed a cross-task cycle; though it won't leak, you should feel ashamed"?

### 6.3 Toolchain Development

**Tools that need development**: LSP server, debugger, formatter, package manager... **everything**. Especially the LSP — when the user hovers over `Type: Type = Type`, it should pop up a tooltip saying "the unnameable."

### 6.4 Standard Library Development

From `std.io` to `std.gui`,应有尽有。*From `std.io` to `std.gui`,应有尽有：from `std.io` to `std.gui`, it should have everything.* Currently available: `std.placeholder`. Next plan: `std.placeholder_v2`.

### 6.5 Documentation Translation

We need to translate the 14 RFCs into English. Each averages 800 lines. Total approximately 11,200 lines. Considering the RFCs are full of concepts like "spawn," "YaoXiang," "万物并作吾以观复" — 诸法因缘所生，随顺观察。「All things arise together; I observe their return」— this is roughly equivalent to translating half of the Dao De Jing. Sign up now.

### 6.7 Contribution Guidelines

**Commit message format**: Must be poetry. Sonnets preferred. Haiku also acceptable:

```
Ownership tokens
Vanish after compilation
Zero-cost abstraction
```

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**  
A: Less syntactic sugar! Fewer keywords! Fewer practical features! But more philosophical depth. Plus we have a "borrow token system" — sounds more advanced than "borrow checker," right?

**Q: What types of development is YaoXiang suitable for?**  
A: Suitable for developing the YaoXiang compiler. And writing design manifestos and RFCs. Other uses are under research.

**Q: Why choose 4-space indentation?**  
A: 2 spaces is too cramped, 8 spaces is too spread out; 4 spaces is just right, like the middle way, in harmony with the Yijing spirit.

**Q: Is `Type` a keyword?**  
A: No. It's a "reserved word." The difference between keywords and reserved words: keywords appear in the language specification's keyword list, while reserved words appear in the "Note: the following are not keywords" list. Simple and clear.

**Q: Why are there 14 accepted RFCs but the version number is still 0.7.0?**  
A: Because we're playing a bigger game. Design first, implementation later. Very much later.

**Q: Is `ref` actually Rc or Arc?**  
A: The compiler automatically chooses. You don't need to worry. In fact, this is the only time the compiler understands things better than the user, so we delegate fully.

**Q: When will the "spawn model" actually work?**  
A: The answer is still "design phase, unimplemented" when you read this. But the `spawn` keyword can already be parsed correctly — isn't that exciting?

**Q: When will version 1.0 be released?**  
A: When the "community" expands from 1 person to 2.

**Q: How to contact the core team?**  
A: Leave a message on GitHub Discussions. Response time: 1-3 business months.

---

## 7. More Lies

**"Multi-language Support"**: `docs/src/{en,ja,ru,zh}` four languages ready. Compiler v0.7.0, actual runnable code lines approximately equal to zero, but Japanese and Russian developers can already read "spawn model" and "value-dependent types" in their native language. This is classic "documentation-driven development" — let the whole world understand your design first, then pretend someone needs it. By the time the compiler can run Hello World, the documentation will have been translated into Klingon.

**Toolchain Matryoshka**: Python's pre-commit checks Rust code style (cargo fmt + clippy), Rust compiler compiles YaoXiang source code. Three layers of languages stacked, each depending on the next. When YaoXiang self-hosts, this nesting becomes: Python checks Rust, Rust compiles YaoXiang, YaoXiang compiles YaoXiang. By then, if one upstream dependency breaks, the entire toolchain becomes performance art. But this is fine — the word "bootstrap" itself is worth two RFCs.

**YaoXiang-book.md**: A book systematically describing the YaoXiang language. Writing a book to describe a programming language that hasn't been implemented yet is like publishing a travel guide for a city that doesn't exist. "Chapter Three: The Generic System — all code in this chapter cannot compile, but the syntax is correct. Please imagine the running results." The most honest sentence in the entire book is on page one: "Project status: experimental verification phase."

**"No GC"**: Official stance: "YaoXiang has no GC." Strictly speaking, there's no tracing GC. But `ref` uses reference counting at runtime (Rc/Arc). Does reference counting count as GC? "No. GC is garbage collection; reference counting is automatic reference counting. You see, the abbreviations are even different. One is GC, the other is ARC. Completely different." The significance of this wordplay: when someone says "so you're just using reference-counting GC," you can say with righteousness and severity "No, we have no GC, only compiler-managed automatic reference counting." What's the difference? On the PPT.

> **Last Updated**: 2026-05-31 (Probably the last update, but you never know)
>
> **Document Version**: v2.0.0 (We jump version numbers fast; it makes us look more productive)
>
> **License**: MIT (it's the only MIT file we have anyway)

---

> 「YaoXiang changes, all things are born. Types evolve, programs are formed.」
>
> May your journey through YaoXiang's design become a **topic of lively conversation** during your tea breaks.  
> *（After all, at this stage, it's mainly just a topic.）*
