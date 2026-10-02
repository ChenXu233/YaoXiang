> **Version**: v2.0.0 (after all, a "formally released" draft is still a release) **Status**:
> Cerebral climax **Author**: Chenxu + the not-yet-gathered "community" **Date**: 2026-05-31 (from
> the future, but the compiler is still stuck in yesterday)

---

> "The Tao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to all things." — _Tao Te Ching_
>
> **Types are the Tao; all things arise from them.** _(Programmers, like ants, are ground down by
> them.)_

---

## I. Why Create YaoXiang? — Because the World Obviously Needs the 514th Language

### 1.1 The Language Gap We Fill

In the long river of programming language history, we have witnessed countless languages being born,
becoming popular, and then thrown into the trash can of history. **But we are different**—we keenly
discovered a stunning void: **there is actually no language that can simultaneously make Rust
enthusiasts feel it's too simple, make Python users feel it's too complex, and make AI models feel
"comfortable" when generating code**.

| Requirement        | Problem with Existing Solutions             | Our Solution (Estimated)                                                                         |
| ------------------ | ------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **Type safety**    | Rust is too strict, TypeScript is too loose | We will create a quantum-superposition type system that is both strict and loose                 |
| **Natural syntax** | Other languages' syntax isn't natural       | Our syntax will be so natural you'll forget you're programming (or maybe you just can't read it) |
| **AI-friendly**    | AI-generated code often has errors          | We will design syntax for AI; humans can use it as a bonus                                       |

### 1.2 The Real Problems We Solve

**Problem 1: Fragmentation of the Type System** We propose "everything is a type," which solves the
troubling philosophical problem of "some things are not types." Now even your code indentation can
be a type (`IndentationLevel<4>`).

**Problem 2: The Either/Or of Memory Safety and Performance** We initially adopted Rust's ownership
model, but found the "borrow checker" too hard to implement. So we had a flash of inspiration—rename
`&T` and `&mut T` from "references" to "tokens," and declare them to be "zero-sized compile-time
permission proofs." Now we don't need a borrow checker, we just need "flow-sensitive liveness
analysis"—sounds completely different, right? If the program has a data race, it must be a problem
with the token brand mechanism.

**Problem 3: The Cognitive Burden of Async Programming** We reinvented the wheel and named it the
"spawn model." Just one `spawn` and the compiler automatically handles all the async details—if it
can't, your code simply isn't "spawn-y" enough.

**Problem 4: The Bottleneck of AI-Assisted Programming** We thoughtfully designed strict indentation
and clear boundaries for AI, ensuring that GPT-7 won't have a schizophrenic breakdown when
generating code. As for whether human programmers can understand it... that's a secondary concern.

### 1.3 The Philosophical Foundation of the Language

YaoXiang's name is derived from the I Ching, which ensures it comes with a built-in mystical buff in
technical discussions. When your code fails to compile, you can say: "The yin and yang are out of
balance; let me cast a hexagram to see."

---

## II. Core Philosophy and Principles — Unquestionable Dogma

### 2.1 Principle 1: Everything is a Type

**Non-negotiable reason**: This way we can use type theory to explain everything, including why
project progress is always delayed.

### 2.2 Principle 2: Strict Structure

**Non-negotiable reason**: 4-space indentation is universal truth. Tab users should be exiled to
Mars.

### 2.3 Principle 3: Zero-Cost Abstractions

**Non-negotiable reason**: Although our abstraction has 7 layers, since it's "zero-cost,"
performance should be roughly on par with hand-written assembly... theoretically.

### 2.4 Principle 4: Immutable by Default

**Non-negotiable reason**: Mutability is the root of all evil. If you need to mutate a variable,
your design is wrong.

### 2.5 Principle 5: Types are Data

**Non-negotiable reason**: This way we can check types at runtime, and then discover... they were
already checked at compile time.

---

## III. Key Innovations and Features — Reinventing What Has Already Been Invented

### 3.1 Innovation 1: Unified Type Syntax

We abolished confusing concepts like `enum`, `struct`, `union`, `trait`, `impl`, and then abolished
the `type` keyword itself. Now everything uses `name: Type = value`. Remember, `Type` is not a
keyword—it's a reserved word. Don't ask what the difference is.

### 3.2 Innovation 2: Constructors are Types

We eliminated the gap between "types" and "values," and created a new gap: "Is this a type
constructor or a value constructor? Oh wait, they use the same syntax now, so it's even more
confusing."

### 3.3 Innovation 3: Curried Method Binding

We implemented method calls through currying. Now you can use `Type.method = function[0]` instead of
the `self` parameter. Obviously more intuitive. `[0]` means "treat the 0th parameter as self"; if
you forget to write `[0]`, the compiler will tell you "this is not a method, this is a regular
function." Simple!

### 3.4 Innovation 4: The Ownership Model (RFC-009 v9)

Five concepts, one gradient: `&T`, `&mut T`, Move, `ref`, `clone()`, `unsafe`. Wait, that's six.
Doesn't matter—`&T` and `&mut T` are "tokens," not "references." The difference? References are a
C++ concept; tokens are zero-sized compile-time type-level permission proofs. When your code fails
to compile, you can say "Dup/Linear type attribute inference failed," and no one dares to argue.

The token system also comes with these advanced features:

- **`freeze`**: "freeze" an `&mut T` into an `&T`. Like putting fresh food in the fridge—you can't
  cook before defrosting. The compiler uses "flow-sensitive liveness analysis" to track freeze
  state. Sounds like an ICU monitor.
- **Brand mechanism**: each token is assigned a unique integer at compile time (brand #N), used for
  anti-counterfeiting. "Sorry sir, your `&Point` token brand #42 doesn't match brand #43 in the
  owner capsule."
- **Cannot cross tasks**: tokens are "compile-time permission proofs" and cannot pass through
  threads. If you need to share across tasks, use `ref`. Why? Because the compiler says no.
  Actually, it's because tokens vanish after compilation—zero-sized type, zero runtime cost, and
  also zero cross-task capability.

Summary: Rust uses 200 pages of The Book to explain the borrow checker. YaoXiang explains everything
in two sentences: "`&T` is copyable, `&mut T` is not." Simple is beautiful.

### 3.5 Innovation 5: The Spawn Model — The Worst Part of the Entire Language

> "All things act together; I observe their return." — _I Ching, Return Hexagram_

The core selling point of the spawn model: **synchronous syntax, asynchronous essence**. In plain
language: your code looks like it executes sequentially, but the runtime automatically parallelizes
it. When to parallelize? How to parallelize? The compiler decides. This is not a concurrency model,
this is a trust exercise.

Let's see what we stuffed into the language to achieve this magic:

**The `spawn` keyword**: marks a function as async. Note—not `async`, but `spawn`. Because `async`
is too mainstream. But `spawn` in Rust means "start a task." Doesn't matter, we redefine it.

**The `@block` annotation**: marks a spawn function to "execute synchronously." Wait—if `spawn` is
async and `@block` makes it sync, why not just not write `spawn`? "Because sometimes you need a
spawn function to run synchronously in some contexts." So a function marked `spawn` may be async or
sync, depending on the caller's mood. This isn't a type system, this is a split personality.

**The `@eager` annotation**: marks an expression that requires "eager evaluation." Because the spawn
model defaults to lazy evaluation—although lazy evaluation isn't implemented yet. So what does
`@eager` actually do right now? It's an IOU: "One day in the future, when lazy evaluation is
implemented, this annotation will prevent the expression from being lazy-evaluated."

**Summary of the three concurrency annotations**:

```
spawn  = this function is async (unless @block is applied)
@block = this spawn function will be sync this time (overrides spawn)
@eager = this expression will not be lazy-evaluated in the future (don't worry about the future for now)
```

If you find this confusing, congratulations—you understand. When your code crashes in parallel, you
can quote the I Ching to appear profound.

### 3.6 Innovation 6: Value-Dependent Types (RFC-011)

Now you can prove at compile time that your array length is prime, that matrix dimensions must
match, and that the result of `factorial(5)` can be used in a type signature. Although this has
nothing to do with writing business logic, "types as propositions, programs as proofs"—isn't that
cool?

### 3.7 Innovation 7: Minimal Keyword Design

Only 18 keywords! 7 fewer than Go! Although each keyword carries 3 times the meaning of a Go
keyword, we win on count. Note: `type` is not a keyword—it was removed in RFC-010. Now you use
`name: Type = value`, where `Type` is a reserved word. What's the difference between a keyword and a
reserved word? Don't ask—if you must, it's a question of the compiler's internal universe hierarchy
Type0/Type1/Type2.

### 3.8 Innovation 8: The Curry-Howard Correspondence — The Universal Explanation

Whenever someone questions a design decision, the standard answer is: "This follows the Curry-Howard
correspondence." Don't understand? It's fine, no one in the community really understands. Roughly,
"types as propositions, programs as proofs," so your code is not just a program—it's a math paper. A
compile error is a proof by contradiction.

The crowning achievement of this philosophy is the easter egg in RFC-010: `Type: Type = Type`. Try
compiling this line—the compiler won't crash; it will output a Zen message along the lines of "The
Tao that can be spoken is not the eternal Tao; the type that can be typed is not the eternal type."
This is YaoXiang's tribute to Girard's paradox, and the only feature the compiler deliberately does
not implement. We call it the "language boundary"—when you reach it, the compiler falls silent here,
and philosophy pauses here.

---

## IV. Initial Syntax Preview — "Looks Like It Works" Code Examples

```yaoxiang
# === Hello World (can be run in your mind) ===
main: () -> Void = {
    print("Hello, 未来的贡献者!")
}

# === Ownership Model: Five Concepts (actually six) ===
Point: Type = { x: Float, y: Float }

p1 = Point(1.0, 2.0)
p2 = p1              # Move. Rest in peace, p1.
p2.print()           # Compiler creates &Point token. Token brand #4201, please accept.
p2.shift(1.0, 1.0)  # Compiler creates &mut Point token. Exclusive! Other tokens stand back!
shared = ref p2      # ref = shared. Compiler auto-selects Rc. Or Arc. You don't need to know.
backup = p2.clone()  # Deep copy. Why not ref? Because ref is not copy, it's sharing. Get it?

# === Unified Syntax: name: type = value ===
# Can you tell which of the following is a type, a function, or a variable?
# Answer: Can't tell. This is the beauty of "unification".
identity: (T: Type) -> ((x: T) -> T) = x
List: (T: Type) -> Type = { data: Array(T), length: Int }

# === Value-Dependent Types: Putting Factorial in Type Signatures ===
factorial: (n: Int) -> Int = {
    # Compiler automatically analyzes parameter decrement, no comments needed
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # Array(Int, 120) type, compile-time computation

# === Spawn Model: spawn + @block + @eager = trinity of chaos ===
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

@block  # This line makes the above spawn function synchronous at this call site
main: () -> Void = {
    data = fetch_data("https://api.example.com")  # Sync? Async? Depends on mood.
}

# @eager: marks "when lazy evaluation is implemented in the future, don't lazy-evaluate here"
result: Int eager = heavy_computation()  # Currently equivalent to writing nothing

# Summary:
# spawn = async (unless @block)
# @block = turns spawn into sync
# @eager = don't do something in the future (nothing happens now)
# Three concepts combined = if else
```

> _The above code runs perfectly in the documentation. Actual compilation results may differ. No,
> they definitely will._

---

## V. Roadmap and Pending Items — Dream List

### 5.0 The RFC Dependency Triangle

Before getting to the roadmap, let's appreciate YaoXiang's most ingenious architectural design—the
RFC love triangle:

```
RFC-009 (Ownership)  →  depends on RFC-010 (Unified Syntax)  →  depends on RFC-011 (Generics)
    ↑                                                                                  │
    └─────────────────────────── depends on ───────────────────────────────────────────┘
```

009 needs 010's syntax, 010 needs 011's generics, 011 needs 009's type system. The three RFCs
mutually presuppose each other. Which one to implement first? "Recommend simultaneous
implementation."—RFC-010, line 141.

This is the Curry-Howard correspondence manifested in real engineering: each RFC is a proposition,
and their dependencies form a logical cycle. Breaking this cycle requires introducing an external
axiom—namely, "let's hardcode the type checker first, deal with it later."

### 5.1 Decided Design Decisions

**No more changes accepted,** unless we change our minds.

### 5.2 Design Issues to Be Discussed

Including "literal syntax," "generic inference," "pattern matching," and other minor details. The
core philosophy is already perfect; these small things can be addressed later.

### 5.3 Implementation Roadmap

```
v0.1: Rust interpreter       ✅
v0.5: Bytecode compiler      🔄 (in progress, has been in progress for 18 months)
v1.0: Production ready       ⏳ (after we find the 10th contributor)
v2.0: Self-hosting           ⏳ (after we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status

- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse that something should follow `spawn`)
- **Type checker**: ✅ 95% (can tell that `42` is of type `Int`, but the universe hierarchy of
  `Type` is still under debate)
- **Ownership token system**: ✅ 100% (design document complete. Implementation? That's the next
  step.)
- **RFC documents**: ✅ 14 accepted (average 800 lines each. Code? What code?)
- **Actually runnable code**: 🔴 0%

---

## VI. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

> _The `authors` field in Cargo.toml: `["YaoXiang Team", "ChenXu2333"]`. Team is listed alongside
> ChenXu2333. Upon verification, Team's current headcount is 1. But the plural form "Team" leaves
> infinite room for imagination._

### 6.1 Design Discussion

**Suitable for**: people who enjoy theoretically debating whether "a monad is a monoid in the
category of endofunctors."

### 6.2 Compiler Implementation

**Suitable for**: those who have spare brain cells and don't mind them being used to implement the
7th memory management model.

Most needed contributions currently:

- **Token conflict detection**: implement "flow-sensitive liveness analysis." Don't worry, despite
  the long name, the principle is simple—track each token's state within the function body: active,
  frozen, moved. Just like tracking three kids in a playground. Except the kids might infinitely
  recurse.
- **Cross-task cycle detection lint**: detect cross-task circular references of `ref`. Default warn,
  configurable to deny. We need someone to decide how harsh the warning wording should be: "Warning:
  cross-task cycle detected" or "Warning: your code forms a cross-task cycle; it won't leak but you
  should be ashamed"?

### 6.3 Toolchain Development

**Tools needed**: LSP server, debugger, formatter, package manager... **everything**. Especially
LSP—when the user hovers on `Type: Type = Type`, a tooltip should pop up reading "the unnameable."

### 6.4 Standard Library

From `std.io` to `std.gui`, we have it all. What we currently have: `std.placeholder`. Next plan:
`std.placeholder_v2`.

### 6.5 Documentation Translation

We need to translate the 14 RFCs into English. Average 800 lines each. Roughly 11,200 lines in
total. Considering the RFCs are full of concepts like "spawn," "YaoXiang," "all things act together,
I observe their return," this is roughly equivalent to translating half of the Tao Te Ching. Sign up
fast.

### 6.7 Contribution Guide

**Commit message format**: must be poetry. Sonnets preferred. Haiku also acceptable:

```
Ownership tokens
Vanish after compilation
Zero-cost abstraction
```

---

## Appendix C: FAQ

**Q: What advantages does YaoXiang have over Rust?** A: Less syntax sugar! Fewer keywords! Fewer
practical features! But more philosophical depth. Plus we have a "borrow token system"—sounds more
advanced than a "borrow checker," right?

**Q: What kind of development is YaoXiang suitable for?** A: Suitable for developing the YaoXiang
compiler. As well as writing design manifestos and RFCs. Other uses TBD.

**Q: Why 4-space indentation?** A: 2-space is too cramped, 8-space is too sparse; 4-space embodies
the doctrine of the mean, in keeping with the spirit of the I Ching.

**Q: Is `Type` a keyword?** A: No. It's a "reserved word." The difference between a keyword and a
reserved word is: keywords appear in the language spec's keyword list, while reserved words appear
in the "Note: the following are not keywords" list. Simple and clear.

**Q: Why are there 14 accepted RFCs but the version is still 0.7.0?** A: Because we're playing the
long game. Design first, implementation later. The "very later" kind of later.

**Q: Is `ref` Rc or Arc?** A: The compiler auto-selects. No need to worry. Actually, this is the
only time the compiler is smarter than the user, so we fully delegate.

**Q: When will the "spawn model" actually work?** A: When you read these words, the answer is still
"design phase, not implemented." But the `spawn` keyword can already be parsed correctly—isn't that
exciting?

**Q: When will version 1.0 be released?** A: When the "community" grows from 1 to 2 people.

**Q: How do I contact the core team?** A: Leave a message on GitHub Discussions. Response time: 1–3
business months.

---

## VII. More Lies

**"Multi-language support"**: `docs/src/{en,ja,ru,zh}` — all four languages are ready. The compiler
is at v0.7.0, and the number of lines of actually runnable code is approximately zero, but
developers in Japan and Russia can already read about the "spawn model" and "value-dependent types"
in their native language. This is classic "documentation-driven development"—first let the whole
world understand your design, then pretend someone needs it. By the time the compiler can run Hello
World, the docs will have been translated into Klingon.

**Toolchain matryoshka**: Python's pre-commit checks Rust's code style (cargo fmt + clippy), the
Rust compiler compiles YaoXiang source. Three layers of language stacked together, each depending on
the next. When YaoXiang self-hosts, the matryoshka becomes: Python checks Rust, Rust compiles
YaoXiang, YaoXiang compiles YaoXiang. By then, if one upstream dependency breaks, the entire
toolchain becomes performance art. But it doesn't matter—the word "self-hosting" alone is worth two
RFCs.

**YaoXiang-book.md**: a book that systematically describes the YaoXiang language. Writing a book to
describe a programming language that hasn't been implemented yet is the equivalent of publishing a
travel guide for a city that doesn't exist. "Chapter 3: The Generics System—the code in this chapter
cannot be compiled, but the syntax is correct. Please imagine the runtime result." The most honest
sentence in the entire book is on the first page: "Project status: experimental verification phase."

**"No GC"**: Official line: "YaoXiang has no GC." Strictly speaking, there's no tracing GC. But
`ref` is reference counting at runtime (Rc/Arc). Does reference counting count as GC? "No. GC is
garbage collection; reference counting is automatic reference counting. See, the abbreviations
aren't even the same. One is GC, the other is ARC. Completely different." The meaning of this
wordplay is: when someone says "isn't this just reference-counting GC?", you can solemnly declare
"no, we don't have GC, only compiler-managed reference counting." Where's the difference? On the
PPT.

> **Last updated**: 2026-05-31 (perhaps the last update, but you'll never know)
>
> **Document version**: v2.0.0 (we bump the version number fast, makes us look productive)
>
> **License**: MIT (after all, right now there's only the MIT file)

---

> "YaoXiang transforms, all things arise. Types evolve, programs come to be."
>
> May YaoXiang's design journey become a **topic of conversation for your leisure time**. _(After
> all, at this stage, it's mainly a talking point.)_

---
