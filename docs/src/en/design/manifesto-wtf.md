# "YaoXiang Design Manifesto" Critique

> **Version**: v2.0.0 (after all, an "official release" draft is still a release)  
> **Status**: Mind orgasm  
> **Author**: ChenXu + the "community" that has yet to coalesce  
> **Date**: 2026-05-31 (from the future, but the compiler is still stuck in yesterday)

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the myriad creatures."  
> — _Tao Te Ching_
>
> **Types are the Dao; the myriad creatures are born from them.**  
> _(Programmers are like ants, all swarming because of it.)_

---

## I. Why Create YaoXiang? — Because the World Obviously Needs the 514th Language

### 1.1 Filling the Language Gap

In the long river of programming language history, we have witnessed countless languages being born,
becoming popular, and then thrown into the history dumpster. But **we're different**—we keenly
identified a stunning void: **there is no language that can simultaneously make Rust enthusiasts
feel it's too simple, Python users feel it's too complex, and AI models feel "comfortable" when
generating code**.

| Need               | Problems with Existing Solutions         | Our Solution (Estimated)                                                                         |
| ------------------ | ---------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **Type Safety**    | Rust is too strict, TypeScript too loose | We will create a quantum-superposition type system that is both strict and loose                 |
| **Natural Syntax** | Other languages' syntaxes aren't natural | Our syntax will be so natural you'll forget you're programming (or maybe you just can't read it) |
| **AI-Friendly**    | AI-generated code often has errors       | We will design syntax for AI; humans can use it on the side                                      |

### 1.2 Practical Problems Solved

**Problem 1: Fragmentation of Type Systems**  
We propose "Everything is a Type," which solves the troubling philosophical problem of "some things
not being types." Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem 2: The Memory Safety vs. Performance Dilemma**  
We initially adopted Rust's ownership model, but found the "borrow checker" too hard to implement.
So we had a flash of inspiration—we renamed `&T` and `&mut T` from "references" to "tokens,"
declaring them to be "zero-sized compile-time permission proofs." Now we don't need a borrow
checker, just "flow-sensitive liveness analysis"—sounds completely different, right? If your program
has a data race, it must be a problem with the token's branding mechanism.

**Problem 3: The Cognitive Burden of Asynchronous Programming**  
We reinvented the wheel and named it the "spawn model." Just one `spawn`, and the compiler handles
all async details automatically—if it can't, then your code isn't "spawn-y" enough.

**Problem 4: The Bottleneck of AI-Assisted Programming**  
We thoughtfully designed strict indentation and clear boundaries for AI, ensuring GPT-7 won't have a
schizophrenic breakdown when generating code. As for whether human programmers can understand it...
that's secondary.

### 1.3 The Philosophical Foundation of the Language

YaoXiang's name comes from the _I Ching_, which ensures it comes with a mysticism buff in technical
discussions. When code won't compile, you can say: "The yin and yang are out of balance; let me cast
a hexagram and see."

---

## II. Core Philosophy and Principles — Incontestable Dogma

### 2.1 Principle 1: Everything is a Type

**Non-negotiable reason**: This way we can explain everything with type theory, including why
project progress is always delayed.

### 2.2 Principle 2: Strict Structure

**Non-negotiable reason**: 4-space indentation is cosmic truth. People who use Tabs should be exiled
to Mars.

### 2.3 Principle 3: Zero-Cost Abstractions

**Non-negotiable reason**: Although our abstraction layer has 7 levels, since it's "zero-cost," the
performance should be roughly equivalent to hand-written assembly... theoretically.

### 2.4 Principle 4: Immutable by Default

**Non-negotiable reason**: Mutability is the root of all evil. If you need to modify a variable,
your design is wrong.

### 2.5 Principle 5: Types are Data

**Non-negotiable reason**: This way we can check types at runtime, then discover... they were
already checked at compile time.

---

## III. Key Innovations and Features — Reinventing What Has Already Been Invented

### 3.1 Innovation 1: Unified Type Syntax

We abolished the confusing concepts of `enum`, `struct`, `union`, `trait`, `impl`, and then we
abolished the `type` keyword itself. Now everything uses `name: Type = value`. Remember, `Type` is
not a keyword—it's a reserved word. Don't ask what the difference is.

### 3.2 Innovation 2: Constructors are Types

We eliminated the gap between "type" and "value," and created a new gap: "Is this a type constructor
or a value constructor? Oh wait, they now use the same syntax, so they're even more
indistinguishable."

### 3.3 Innovation 3: Curried Method Binding

We implemented method calls through currying. Now you can use `Type.method = function[0]` instead of
the `self` parameter. Obviously more intuitive. `[0]` means "treat the 0th argument as self." If you
forget to write `[0]`, the compiler will tell you "this is not a method, this is a regular
function." Simple!

### 3.4 Innovation 4: Ownership Model (RFC-009 v9)

Five concepts, one gradient: `&T`, `&mut T`, Move, `ref`, `clone()`, `unsafe`. Wait, that's six.
Whatever—`&T` and `&mut T` are "tokens," not "references." What's the difference? References are a
C++ concept; tokens are compile-time zero-sized type-level permission proofs. When your code won't
compile, you can say "type attribute Dup/Linear inference failed," and no one dares argue.

The token system comes with these advanced features:

- **`freeze`**: "Freezes" `&mut T` into `&T`. Like putting fresh food in the fridge—you can't cook
  it before thawing. The compiler tracks freeze state with "flow-sensitive liveness analysis."
  Sounds like an ICU monitor.
- **Branding mechanism**: Each token is assigned a unique integer at compile time (Brand #N) to
  prevent counterfeiting. "I'm sorry sir, your `&Point` token brand #42 does not match brand #43 in
  the owner's capsule."
- **Cannot cross tasks**: Tokens are "compile-time permission proofs" and cannot cross threads. If
  you need to share across tasks, use `ref`. Why? Because the compiler says so. Actually it's
  because tokens disappear after compilation—zero-sized types, zero runtime overhead, also zero
  cross-task capability.

Summary: Rust uses 200 pages of The Book to explain the borrow checker. YaoXiang uses "`&T` is
copyable, `&mut T` is not copyable"—two sentences to explain everything. Simplicity is beauty.

### 3.5 Innovation 5: The Spawn Model—The Worst Part of the Entire Language

> "The myriad creatures arise together; I observe their return." —_I Ching, Fu Hexagram_

The core selling point of the spawn model: **synchronous syntax, asynchronous essence**. In plain
words: your code looks sequential, but the runtime will automatically parallelize it. When does it
parallelize? How does it parallelize? The compiler decides. This isn't a concurrency model; this is
a trust game.

Let's look at what we stuffed into the language to achieve this magic:

**`spawn` keyword**: Marks a function as asynchronous. Note—not `async`, but `spawn`. Because
`async` is too mainstream. But in Rust, `spawn` means "launch a task." Whatever, we redefine it.

**`@block` annotation**: Marks a spawn function to "execute synchronously." Wait—if `spawn` is
async, and `@block` makes it sync, why not just not write `spawn`? "Because sometimes you need a
spawn function to run synchronously in certain contexts." So a function marked with `spawn` might be
async or sync, depending on the caller's mood. This isn't a type system; this is dissociative
identity disorder.

**`@eager` annotation**: Marks expressions that need "eager evaluation." Because the spawn model
defaults to lazy evaluation—even though lazy evaluation isn't implemented yet. So what does `@eager`
actually do? It's an IOU: "Someday, when lazy evaluation is implemented, this annotation will
prevent the expression from being lazily evaluated."

**Summary of the concurrency model's three annotations**:

```
spawn  = This function will be async (unless @blocked)
@block = This spawn function will be sync this time (overrides spawn)
@eager = This expression will not be lazily evaluated in the future (don't worry about the future for now)
```

If you find this confusing, congratulations—you understand. When your parallel code crashes, you can
cite the _I Ching_ and look profound.

### 3.6 Innovation 6: Value-Dependent Types (RFC-011)

Now you can prove at compile time that your array length is prime, that matrix dimensions must
match, that the result of factorial(5) can be used in type signatures. Although this has nothing to
do with writing business logic, "types are propositions, programs are proofs"—isn't that cool?

### 3.7 Innovation 7: Minimal Keyword Design

Only 17 keywords! 8 fewer than Go! Although each keyword's meaning is 3 times more complex than Go's
keywords, we win on count. Note: `type` is not a keyword—it was removed in RFC-010. Now you use
`name: Type = value`, where `Type` is a reserved word. The difference between a keyword and a
reserved word? Don't ask, just know it's about the compiler's internal universe hierarchy
Type0/Type1/Type2.

### 3.8 Innovation 8: Curry-Howard Correspondence—The Universal Justification

Whenever someone questions a design decision, the standard answer is: "This follows the Curry-Howard
correspondence." Don't understand? No problem, no one in the community really does. The gist is
"types are propositions, programs are proofs," so your code isn't just a program—it's a math paper.
Compile errors are proofs by contradiction.

The highest achievement of this philosophy is the easter egg in RFC-010: `Type: Type = Type`. Try
compiling this line; the compiler won't crash—it will output a Zen message along the lines of "The
Dao that can be named is not the eternal Dao; the type that can be typed is not the eternal type."
This is YaoXiang's tribute to Girard's paradox, and the only feature the compiler deliberately does
not implement. We call this the "language boundary"—when you reach it, the compiler falls silent,
and philosophy pauses here.

---

## IV. Preliminary Syntax Preview — "Looks Like It Could Work" Code Examples

```yaoxiang
# === Hello World (can run in your mind) ===
main: () -> Void = {
    print("Hello, future contributor!")
}

# === Ownership Model: Five Concepts (Actually Six) ===
Point: Type = { x: Float, y: Float }

p1 = Point(1.0, 2.0)
p2 = p1              # Move. p1, rest in peace.
p2.print()           # Compiler creates &Point token. Token brand #4201, please acknowledge.
p2.shift(1.0, 1.0)  # Compiler creates &mut Point token. Exclusive! Other tokens stand back!
shared = ref p2      # ref = share. Compiler auto-selects Rc. Or Arc. You don't need to know.
backup = p2.clone()  # Deep copy. Why not ref? Because ref isn't copy, it's share. Get it?

# === Unified Syntax: name: type = value ===
# Can you tell which of the following is a type, a function, or a variable?
# Answer: No, you can't. This is the beauty of "unification."
identity: (T: Type) -> ((x: T) -> T) = x
List: (T: Type) -> Type = { data: Array(T), length: Int }

# === Value-Dependent Types: Put Factorial in the Type Signature ===
factorial: (n: Int) -> Int = {
    # Compiler auto-analyzes parameter decrement, no comments needed
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # Array(Int, 120) type, compile-time computed

# === Spawn Model: spawn + @block + @eager = Trinitarian Chaos ===
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

@block  # This line makes the spawn function above synchronous in this call
main: () -> Void = {
    data = fetch_data("https://api.example.com")  # Sync? Async? Depends on the mood.
}

# @eager: Marks "when lazy evaluation is implemented in the future, don't lazily evaluate here"
result: Int eager = heavy_computation()  # Currently the same as not writing it

# Summary:
# spawn = async (unless @block)
# @block = make spawn synchronous
# @eager = in the future, don't do something (right now, nothing happens)
# Three concepts combined = if else
```

> _The above code runs fine in the documentation. Actual compilation results may differ. No, they
> definitely will._

---

## V. Roadmap and Pending Items — Dream List

### 5.0 The RFC Dependency Triangle

Before looking at the roadmap, let's appreciate YaoXiang's most ingenious architectural design—the
RFC love triangle:

```
RFC-009 (Ownership)  →  depends on RFC-010 (Unified Syntax)  →  depends on RFC-011 (Generics)
    ↑                                                                              │
    └────────────────────── depends on ────────────────────────────────────────────┘
```

009 needs 010's syntax, 010 needs 011's generics, 011 needs 009's type system. Three RFCs depend on
each other as prerequisites. Which one to implement first? "Recommend implementing
synchronously."—RFC-010, line 141.

This is the Curry-Howard correspondence in real engineering: each RFC is a proposition, and their
dependencies form a logical cycle. Breaking this cycle requires introducing an external axiom—i.e.,
"we hardcode the type checker first, deal with it later."

### 5.1 Decided Design Decisions

**No longer accepting changes**, unless we change our minds.

### 5.2 Design Issues to be Discussed

Including trivial details like "literal syntax," "generic inference," "pattern matching," etc. The
core philosophy is already perfect; these small things can come later.

### 5.3 Implementation Roadmap

```
v0.1: Rust interpreter         ✅
v0.5: Bytecode compiler        🔄 (in progress, has been for 18 months)
v1.0: Production ready         ⏳ (when we find the 10th contributor)
v2.0: Self-hosting             ⏳ (after we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status

- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse that something should follow `spawn`)
- **Type checker**: ✅ 95% (can determine `42` is `Int`, but the universe hierarchy of `Type` is
  still under debate)
- **Ownership token system**: ✅ 100% (design document complete. Implementation? That's the next
  step.)
- **RFC documents**: ✅ 14 accepted (average 800 lines each. Code? What code?)
- **Actually runnable code**: 🔴 0%

---

## VI. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

> _The authors listed in Cargo.toml: ["YaoXiang Team", "ChenXu2333"]. Team and ChenXu2333 are listed
> side by side. Upon investigation, Team currently has 1 member. But the plural form "Team" leaves
> infinite room for imagination._

### 6.1 Design Discussion

**Suited for**: People who enjoy debating "whether a monad is a monoid in the category of
endofunctors" in theory.

### 6.2 Compiler Implementation

**Suited for**: Those with spare brain cells, who don't mind them being used to implement the 7th
memory management model.

Current most needed contributions:

- **Token conflict detection**: Implement "flow-sensitive liveness analysis." Don't worry, the name
  is long, but the principle is simple—just track the state of each token in a function body:
  active, frozen, moved. Like tracking the positions of three kids in a playground. Except the kids
  might infinitely recurse.
- **Cross-task cycle detection lint**: Detect cross-task cyclic references in `ref`. Default warn,
  configurable deny. We need someone to decide how harsh the warn wording should be: "Warning:
  cross-task cycle detected" or "Warning: your code formed a cross-task cycle; it won't leak but you
  should be ashamed"?

### 6.3 Toolchain Development

**Tools needed**: LSP server, debugger, formatter, package manager... **everything**. Especially
LSP—when users hover over `Type: Type = Type`, a tooltip reading "the unspeakable" should pop up.

### 6.4 Standard Library Construction

From `std.io` to `std.gui`, we have everything. Currently existing: `std.placeholder`. Next plan:
`std.placeholder_v2`.

### 6.5 Documentation Translation

We need to translate 14 RFCs into English. Each averages 800 lines. Total approximately 11,200
lines. Considering the RFCs are full of concepts like "spawn," "YaoXiang," and "the myriad creatures
arise together, I observe their return," this is roughly equivalent to translating half of the _Tao
Te Ching_. Sign up now.

### 6.7 Contribution Guidelines

**Commit message format**: Must be poetry. Sonnets preferred. Haiku also acceptable:

```
Ownership tokens
Disappear after compilation
Zero-cost abstract
```

---

## Appendix C: Frequently Asked Questions

**Q: What advantages does YaoXiang have over Rust?**  
A: Less syntax sugar! Fewer keywords! Fewer practical features! But more philosophical depth. Plus
we have a "borrow token system"—sounds more advanced than "borrow checker," right?

**Q: What kind of development is YaoXiang suitable for?**  
A: Suitable for developing the YaoXiang compiler. And writing design manifestos and RFCs. Other uses
TBD.

**Q: Why 4-space indentation?**  
A: 2 spaces is too cramped, 8 spaces is too sparse, 4 spaces is the golden mean, in keeping with the
spirit of the _I Ching_.

**Q: Is `Type` a keyword?**  
A: No. It's a "reserved word." The difference between a keyword and a reserved word is: keywords
appear in the language spec's keyword list, while reserved words appear in the "Note: the following
are not keywords" list. Crystal clear.

**Q: Why are there 14 accepted RFCs but the version is still 0.7.0?**  
A: Because we're playing a long game. Design first, implementation later. A very long "later."

**Q: Is `ref` Rc or Arc?**  
A: The compiler auto-selects. You don't need to worry. In fact, this is the only time the compiler
is smarter than the user, so we fully delegate.

**Q: When will the "spawn model" actually work?**  
A: When you read this line, the answer is still "design phase, not implemented." But the `spawn`
keyword can already be correctly parsed. Isn't that exciting?

**Q: When will version 1.0 be released?**  
A: When the "community" expands from 1 to 2 people.

**Q: How to contact the core team?**  
A: Leave a message on GitHub Discussions. Response time: 1-3 business months.

---

## VII. More Lies

**"Multi-language support"**: `docs/src/{en,ja,ru,zh}`—four languages complete. Compiler v0.7.0, the
number of actually runnable lines of code is approximately zero, but Japanese and Russian developers
can already read about "spawn model" and "value-dependent types" in their native language. This is
classic "documentation-driven development"—first let the whole world understand your design, then
pretend someone needs it. By the time the compiler can run Hello World, the documentation will
already be translated into Klingon.

**Toolchain matryoshka**: Python's pre-commit checks Rust's code style (cargo fmt + clippy), Rust
compiler compiles YaoXiang source. Three layers of language stacked together, each depending on the
next. When YaoXiang self-hosts, the matryoshka becomes: Python checks Rust, Rust compiles YaoXiang,
YaoXiang compiles YaoXiang. By then, if one upstream dependency breaks, the entire toolchain becomes
performance art. But it doesn't matter—the word "self-hosting" alone is worth two RFCs.

**YaoXiang-book.md**: A book systematically describing the YaoXiang language. Writing a book to
describe a programming language that hasn't been implemented is equivalent to publishing a travel
guide to a city that doesn't exist. "Chapter 3: The Generic System—The code in this chapter cannot
be compiled, but the syntax is correct. Please imagine the running results." The most honest
sentence in the entire book is on the first page: "Project status: experimental verification phase."

**"No GC"**: Official stance: "YaoXiang has no GC." Strictly speaking, there's no tracing GC. But
`ref` is reference counting (Rc/Arc) at runtime. Does reference counting count as GC? "No. GC is
garbage collection, reference counting is automatic reference counting. See, the abbreviations are
different. One is GC, the other is ARC. Completely different." The meaning of this wordplay: when
someone says "isn't this just reference counting GC?", you can justifiably say "No, we don't have
GC, only compiler-managed automatic reference counting." What's the difference? On the PPT.

> **Last updated**: 2026-05-31 (possibly the last update, but you'll never know)
>
> **Document version**: v2.0.0 (we bump the version fast; it makes progress look faster)
>
> **License**: MIT (anyway, right now only the MIT file exists)

---

> "YaoXiang transforms, the myriad creatures arise. Types evolve, programs come to be."
>
> May the design journey of YaoXiang become a **delightful topic of conversation** for your leisure
> moments.  
> _(After all, at this stage, it's mainly a topic of conversation.)_

---
