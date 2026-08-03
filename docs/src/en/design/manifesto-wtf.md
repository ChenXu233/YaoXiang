# "YaoXiang Design Manifesto" Sharp Critique

> **Version**: v2.0.0 (after all, a "formally released" draft is still a release)  
> **Status**: Intra-cranial climax  
> **Author**: ChenXu + "Community" not yet assembled  
> **Date**: 2026-05-31 (from the future, but the compiler is still in yesterday)

---

> "The Tao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth
> to the myriad things."  
> — _Tao Te Ching_
>
> **Types are the Tao; all things are born from them.**  
> _(Programmers are like ants, all writhing from this.)_

---

## I. Why Create YaoXiang? — Because the World Obviously Needs a 514th Language

### 1.1 Filling the Language Gap

In the long river of programming language history, we have witnessed countless languages being born,
becoming popular, and then thrown into the trash can of history. But **we are different**—we keenly
discovered a shocking void: **there is simply no language that can simultaneously make Rust
enthusiasts feel it's too simple, make Python users feel it's too complex, and make AI models feel
"comfortable" when generating code.**

| Need               | Problems with Existing Solutions           | Our Solution (Estimated)                                                                           |
| ------------------ | ------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| **Type Safety**    | Rust is too strict, TypeScript too loose   | We will create a quantum superposition type system that is both strict and loose                   |
| **Natural Syntax** | Other languages' syntax isn't natural      | Our syntax will be so natural that you forget you're programming (or maybe you just can't read it) |
| **AI-Friendly**    | AI often makes mistakes in code generation | We will design the syntax for AI; humans can use it as a bonus                                     |

### 1.2 Actual Problems Solved

**Problem One: Fragmentation of Type Systems**  
We propose "everything is a type," which resolves the troubling philosophical question of "some
things are not types." Now even your code indentation can be a type (`IndentationLevel<4>`).

**Problem Two: The Binary Choice Between Memory Safety and Performance**  
We initially adopted Rust's ownership model, but found the "borrow checker" too hard to implement.
So a flash of inspiration—rename `&T` and `&mut T` from "references" to "tokens," and declare them
as "zero-sized compile-time permission proofs." Now no borrow checker is needed, only
"flow-sensitive liveness analysis"—sounds completely different, right? If your program has a data
race, it must be a problem with the token branding mechanism.

**Problem Three: The Cognitive Load of Asynchronous Programming**  
We reinvented the wheel and named it the "spawn model." Just one `spawn`, and the compiler will
automatically handle all async details—if it can't, then your code isn't "spawn-y" enough.

**Problem Four: The Bottleneck of AI-Assisted Programming**  
We thoughtfully designed strict indentation and clear boundaries for AI, ensuring that GPT-7 won't
have a schizophrenic episode when generating code. As for whether human programmers can understand
it... that's secondary.

### 1.3 The Philosophical Foundation of the Language

YaoXiang's name comes from the _I Ching_ (Book of Changes), which ensures it comes with a mystical
buff in technical discussions. When your code won't compile, you can say: "The yin and yang are out
of balance; let me cast a hexagram to see."

---

## II. Core Philosophy and Principles — Inarguable Holy Writ

### 2.1 Principle One: Everything is a Type

**Uncompromising Reason**: This way we can use type theory to explain everything, including why
project timelines are always delayed.

### 2.2 Principle Two: Strict Structure

**Uncompromising Reason**: 4-space indentation is universal truth. Those who use Tabs should be
exiled to Mars.

### 2.3 Principle Three: Zero-Cost Abstractions

**Uncompromising Reason**: Although our abstraction layers number 7, since they are "zero-cost,"
performance should be roughly equivalent to hand-written assembly... theoretically.

### 2.4 Principle Four: Immutable by Default

**Uncompromising Reason**: Mutability is the root of all evil. If you need to modify a variable,
your design is wrong.

### 2.5 Principle Five: Types are Data

**Uncompromising Reason**: This way we can check types at runtime, and then discover... they were
already checked at compile time.

---

## III. Key Innovations and Features — Reinventing What Has Already Been Invented

### 3.1 Innovation One: Unified Type Syntax

We abolished the confusing concepts of `enum`, `struct`, `union`, `trait`, `impl`, and then we even
abolished the `type` keyword itself. Now everything uses `name: Type = value`. Remember, `Type` is
not a keyword—it is a reserved word; don't ask what the difference is.

### 3.2 Innovation Two: Constructors are Types

We eliminated the chasm between "types" and "values," creating a new chasm: "Is this a type
constructor or a value constructor? Oh wait, they now use the same syntax, even more
indistinguishable."

### 3.3 Innovation Three: Curried Method Binding

We implemented method calls through currying. Now you can use `Type.method = function[0]` instead of
the `self` parameter. Obviously more intuitive. `[0]` means "treat the 0th argument as self"; if you
forget to write `[0]`, the compiler will tell you "this is not a method, this is a regular
function." Simple!

### 3.4 Innovation Four: Ownership Model (RFC-009 v9)

Five concepts, one gradient: `&T`, `&mut T`, Move, `ref`, `clone()`, `unsafe`. Wait, that's six.
Never mind—`&T` and `&mut T` are "tokens," not "references." What's the difference? References are a
C++ concept; tokens are compile-time zero-sized type-level permission proofs. When your code won't
compile, you can say "the type property Dup/Linear inference failed," and no one dares to argue.

The token system also comes with these advanced features:

- **`freeze`**: "Freezes" an `&mut T` into an `&T`. Like putting fresh food in the fridge—can't be
  cooked before defrosting. The compiler uses "flow-sensitive liveness analysis" to track freeze
  state; sounds like an ICU monitor.
- **Branding mechanism**: Each token is assigned a unique integer (brand #N) at compile time to
  prevent counterfeiting. "Sorry sir, your `&Point` token brand #42 does not match the brand #43 in
  the owner's capsule."
- **Cannot cross tasks**: Tokens are "compile-time permission proofs" and cannot pass through
  threads. If you need to share across tasks, please use `ref`. Why? Because the compiler says no.
  Actually, it's because tokens disappear after compilation—zero-sized type, zero runtime overhead,
  and also zero cross-task capability.

Summary: Rust uses 200 pages of The Book to explain the borrow checker. YaoXiang uses "`&T` is
copyable, `&mut T` is not" in two sentences to explain everything. Simplicity is beauty.

### 3.5 Innovation Five: The spawn Model — The Worst Part of the Entire Language

> "The myriad things arise together; by this I observe the return." — _I Ching · Return (Fù)
> Hexagram_

The core selling point of the spawn model: **synchronous syntax, asynchronous essence**. In plain
language: your code looks like it runs sequentially, but at runtime it automatically runs in
parallel. When does it run in parallel? How? The compiler decides. This isn't a concurrency model;
this is a trust game.

Let's see what we stuffed into the language to achieve this magic:

**`spawn` keyword**: Marks a function as asynchronous. Note—not `async`, but `spawn`. Because
`async` is too mainstream. But `spawn` in Rust means "start a task." No worries, we redefine it.

**`@block` annotation**: Marks a spawn function that should "execute synchronously." Wait—if `spawn`
is asynchronous, and `@block` makes it synchronous, then why not just not write `spawn`? "Because
sometimes you need a spawn function to run synchronously in certain contexts." So a function
annotated with `spawn` may be asynchronous or synchronous, depending on the caller's mood. This
isn't a type system; this is multiple personality disorder.

**`@eager` annotation**: Marks an expression that needs "eager evaluation." Because the spawn model
defaults to lazy evaluation—even though lazy evaluation hasn't been implemented yet. So what does
`@eager` actually do right now? It's an IOU: "Someday, when lazy evaluation is implemented, this
annotation will keep this expression from being lazily evaluated."

**Summary of the concurrency model's three annotations**:

```
spawn  = This function will be asynchronous (unless @blocked)
@block = This spawn function will be synchronous this time (overrides spawn)
@eager = This expression will not be lazily evaluated in the future (don't worry about the future for now)
```

If you find this confusing, congratulations—you understand. When your parallel code crashes, you can
quote the _I Ching_ to sound profound.

### 3.6 Innovation Six: Value-Dependent Types (RFC-011)

Now you can prove at compile time that your array length is prime, that matrix dimensions must
match, and that the result of factorial(5) can be used in type signatures. Although this has nothing
to do with writing business logic, "types as propositions, programs as proofs"—isn't that cool?

### 3.7 Innovation Seven: Minimal Keyword Design

Only 17 keywords! 8 fewer than Go! Although each keyword's meaning is 3 times more complex than Go's
keywords, we win on count. Note: `type` is not a keyword—it was removed in RFC-010. Now you use
`name: Type = value`, where `Type` is a reserved word. What's the difference between keywords and
reserved words? Don't ask; the answer lies in the compiler's internal universe hierarchy
Type0/Type1/Type2.

### 3.8 Innovation Eight: Curry-Howard Correspondence — The Universal Explanation

Whenever someone questions a design decision, the standard reply is: "This follows the Curry-Howard
correspondence." Don't understand? No problem, no one in the community really does. The gist is
"types are propositions, programs are proofs," so your code isn't just a program—it's a mathematical
paper. Compilation errors are proofs by contradiction.

The crowning achievement of this philosophy is RFC-010's Easter egg: `Type: Type = Type`. Try
compiling this line of code, the compiler won't crash—it will output a Zen-like message along the
lines of "The Tao that can be spoken is not the eternal Tao, the type that can be typed is not the
eternal type." This is YaoXiang's tribute to Girard's paradox, and the only feature the compiler
intentionally does not implement. We call it the "language boundary"—when you reach it, the compiler
falls silent, and philosophy pauses here.

---

## IV. Preliminary Syntax Preview — Code Examples That "Look Like They Could Work"

```yaoxiang
# === Hello World (can run in your mind) ===
main: () -> Void = {
    print("Hello, future contributors!")
}

# === Ownership Model: Five Concepts (actually six) ===
Point: Type = { x: Float, y: Float }

p1 = Point(1.0, 2.0)
p2 = p1              # Move. p1, rest in peace.
p2.print()           # Compiler creates &Point token. Token brand #4201, please keep it.
p2.shift(1.0, 1.0)   # Compiler creates &mut Point token. Exclusive! Other tokens, retreat!
shared = ref p2      # ref = shared. Compiler automatically picks Rc. Or Arc. You don't need to know.
backup = p2.clone()  # Deep copy. Why not ref? Because ref isn't copy, it's share. Get it?

# === Unified Syntax: name: type = value ===
# Can you tell which of the following is a type, which is a function, and which is a variable?
# Answer: You can't. This is the beauty of "unification."
identity: (T: Type) -> ((x: T) -> T) = x
List: (T: Type) -> Type = { data: Array(T), length: Int }

# === Value-Dependent Types: Writing Factorial in Type Signatures ===
factorial: (n: Int) -> Int = {
    # Compiler automatically analyzes parameter decrement, no comments needed
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
vec: Vec(factorial(5)) = Vec(120)()  # Vec(120) type, computed at compile time

# === spawn Model: spawn + @block + @eager = Trinity of Chaos ===
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

@block  # This line makes the above spawn function synchronous at this call site
main: () -> Void = {
    data = fetch_data("https://api.example.com")  # Sync? Async? Depends on mood.
}

# @eager: Marks "don't lazily evaluate here when lazy evaluation is implemented in the future"
result: Int eager = heavy_computation()  # Currently equivalent to writing nothing

# Summary:
# spawn = asynchronous (unless @block)
# @block = makes spawn synchronous
# @eager = won't do something in the future (nothing happens now)
# Three concepts combined = if else
```

> _The above code runs perfectly in the document. Actual compilation results may vary. No, they will
> definitely vary._

---

## V. Roadmap and Pending Items — The Dream List

### 5.0 The RFC Dependency Triangle

Before learning the roadmap, let's appreciate YaoXiang's most ingenious architectural design—the RFC
love triangle:

```
RFC-009 (Ownership) → depends on RFC-010 (Unified Syntax) → depends on RFC-011 (Generics)
    ↑                                                                      │
    └───────────────────── depends on ──────────────────────────────────────┘
```

009 needs 010's syntax, 010 needs 011's generics, 011 needs 009's type system. Three RFCs depend on
each other. Which to implement first? "Recommend implementing simultaneously."—RFC-010, line 141.

This is how the Curry-Howard correspondence manifests in real engineering: each RFC is a
proposition, and their dependencies form a logical loop. Breaking this loop requires introducing an
external axiom—that is, "let's just hardcode the type checker for now, and figure it out later."

### 5.1 Decided Design Decisions

**No more changes accepted**, unless we change our minds.

### 5.2 Design Topics Under Discussion

Including trivial details like "literal syntax," "generics inference," "pattern matching." The core
philosophy is already perfect; these minor things can be addressed slowly.

### 5.3 Implementation Roadmap

```
v0.1: Rust interpreter          ✅
v0.5: Bytecode compiler         🔄 (in progress, has been for 18 months)
v1.0: Production-ready          ⏳ (waiting to find the 10th contributor)
v2.0: Self-hosting              ⏳ (after we solve the time travel problem in v1.0)
```

### 5.4 Current Implementation Status

- **Lexer**: ✅ 100% (can recognize the word `spawn`)
- **Parser**: ✅ 100% (can parse that something should follow `spawn`)
- **Type checker**: ✅ 95% (can determine that `42` is of type `Int`, but the universe hierarchy of
  `Type` is still under debate)
- **Ownership token system**: ✅ 100% (design document complete. Implementation? That's the next
  step.)
- **RFC documents**: ✅ 14 accepted (average 800 lines each. Code? What code?)
- **Actual runnable code**: 🔴 0%

---

## VI. How to Contribute — Please Bring Your Time, Enthusiasm, and Lowered Expectations

> _Recorded in Cargo.toml: authors: ["YaoXiang Team", "ChenXu2333"]. Team is listed alongside
> ChenXu2333. Upon verification, the current size of Team is 1. But the plural form "Team" leaves
> infinite room for imagination._

### 6.1 Design Discussion

**Suitable for**: People who like to debate theoretically whether "a monad is a monoid in the
category of endofunctors."

### 6.2 Compiler Implementation

**Suitable for**: Those with spare brain cells, who don't mind using them to implement the 7th
memory management model.

Current most-needed contributions:

- **Token conflict detection**: Implement "flow-sensitive liveness analysis." Don't worry, although
  the name is long, the principle is simple—just track each token's state within the function body:
  active, frozen, moved. Like tracking three kids' positions in a playground. Except the kids might
  recurse infinitely.
- **Cross-task cycle detection lint**: Detect `ref` cross-task circular references. Default warn,
  configurable to deny. We need someone to decide: how severe should the warning wording be?
  "Warning: cross-task cycle detected" or "Warning: your code has formed a cross-task cycle, it
  won't leak but you should feel ashamed"?

### 6.3 Toolchain Development

**Tools needed**: LSP server, debugger, formatter, package manager... **everything**. Especially the
LSP—when users hover over `Type: Type = Type`, a tooltip reading "the unnameable" should pop up.

### 6.4 Standard Library Construction

From `std.io` to `std.gui`, everything you need. What currently exists: `std.placeholder`. Next
plan: `std.placeholder_v2`.

### 6.5 Documentation Translation

We need to translate the 14 RFCs into English. Average 800 lines each. About 11,200 lines total.
Considering the RFCs are full of concepts like "spawn," "YaoXiang," and "the myriad things arise
together; by this I observe the return," this is roughly equivalent to translating half of the _Tao
Te Ching_. Sign up quickly.

### 6.7 Contribution Guidelines

**Commit message format**: Must be poetry. Sonnets preferred. Haiku also acceptable:

```
Ownership token
Vanishes after compilation
Zero-cost abstraction
```

---

## Appendix C: Frequently Asked Questions

**Q: What are the advantages of YaoXiang compared to Rust?**  
A: Fewer syntax sugars! Fewer keywords! Fewer practical features! But more philosophical depth.
Also, we have a "token system"—sounds more advanced than "borrow checker," right?

**Q: What kind of development is YaoXiang suitable for?**  
A: Suitable for developing the YaoXiang compiler. And writing design manifestos and RFCs. Other uses
remain to be studied.

**Q: Why choose 4-space indentation?**  
A: 2 spaces is too tight, 8 spaces is too loose, 4 spaces is the doctrine of the mean, in keeping
with the spirit of the _I Ching_.

**Q: Is `Type` a keyword?**  
A: No. It is a "reserved word." The difference between keywords and reserved words is: keywords
appear in the language specification's keyword list, while reserved words appear in the "Note: the
following are not keywords" list. Crystal clear.

**Q: Why are there 14 accepted RFCs but the version number is still 0.7.0?**  
A: Because we're playing a grand game of chess. Design first, implementation later. Very much later.

**Q: Is `ref` Rc or Arc?**  
A: The compiler chooses automatically. You don't need to worry about it. In fact, this is the only
time the compiler is smarter than the user, so we fully delegate.

**Q: When will the "spawn model" actually work?**  
A: The moment you read this line, the answer is still "design phase, not implemented." But the
`spawn` keyword can be parsed correctly now—doesn't that excite you?

**Q: When will version 1.0 be released?**  
A: When the "community" expands from 1 person to 2 people.

**Q: How do I contact the core team?**  
A: Leave a message on GitHub Discussions. Response time: 1-3 business months.

---

## VII. More Lies

**"Multi-language support"**: `docs/src/{en,ja,ru,zh}` — all four languages present. Compiler
v0.7.0, the actual number of runnable code lines is approximately zero, but developers from Japan
and Russia can already read about the "spawn model" and "value-dependent types" in their native
language. This is classic "document-driven development"—first let the whole world understand your
design, then pretend someone needs it. By the time the compiler can run Hello World, the
documentation will have been translated into Klingon.

**Toolchain Russian nesting doll**: Python's pre-commit checks Rust's code style (cargo fmt +
clippy), Rust compiler compiles YaoXiang source. Three layers of language stacked together, each
depending on the next. When YaoXiang self-hosts, the nesting doll becomes: Python checks Rust, Rust
compiles YaoXiang, YaoXiang compiles YaoXiang. At that point, if an upstream dependency breaks, the
entire toolchain becomes performance art. But that's okay—the word "self-hosting" alone is worth two
RFCs.

**YaoXiang-book.md**: A book systematically describing the YaoXiang language. Writing a book to
describe a programming language that hasn't been implemented yet is equivalent to publishing a
travel guide for a city that doesn't exist. "Chapter 3: The Generics System—The code in this chapter
cannot be compiled, but the syntax is correct. Please imagine the runtime result." The most honest
sentence in the entire book is on the first page: "Project Status: Experimental Verification Phase."

**"No GC"**: Official statement: "YaoXiang has no GC." Strictly speaking, no tracing GC. But `ref`
at runtime is reference counting (Rc/Arc). Does reference counting count as GC? "No. GC is garbage
collection, reference counting is automatic reference counting. You see, the abbreviations are
different. One is GC, the other is ARC. Completely different." The significance of this word game
is: when someone says "isn't this just reference-counted GC?", you can solemnly declare "No, we have
no GC, only compiler-managed automatic reference counting." What's the difference? On the PPT.

> **Last updated**: 2026-05-31 (possibly the last update, but you never know)
>
> **Document version**: v2.0.0 (we bump the version number fast, it makes us look more productive)
>
> **License**: MIT (anyway, there's only the MIT file for now)

---

> "YaoXiang transforms; the myriad things are born. Types evolve; programs are made."
>
> May the design journey of YaoXiang become **a topic of lively conversation** for you over tea and
> after meals.  
> _(After all, at this stage, it is mainly a topic of conversation.)_

---
