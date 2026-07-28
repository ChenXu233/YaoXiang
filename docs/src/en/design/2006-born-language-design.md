# A Language Design Perspective from Someone Born in 2006

> When Rust was being conceived, I was just born; when Rust matured, I was in my youth; and in the
> next decade, I am precisely the one who can create a language belonging to our generation.

## Introduction: The Passing of a Generational Mission

2006 is the year the Rust programming language was born, and also the year I came into this world.
Nineteen years later, when I began designing and implementing YaoXiang (爻象), I realized this was
not merely a coincidence of time, but a passing of a generational mission.

Rust solved the pain points of the 2000s: memory safety, concurrency safety. It is the answer given
by a generation of engineers after struggling in the quagmire of C/C++. But each generation has its
own problems, and each generation needs its own tools.

This article is not a technical specification document, but a generational manifesto. The question
it seeks to answer is: **Why do developers of our generation need our own language? How does
YaoXiang respond to our needs?**

---

## I. The Generation Gap — Why Existing Languages Make Us Feel "Culturally Disconnected"

### 1.1 Those "Counter-intuitive" Designs

When I first learned Rust, I was tormented to death by its borrow checker. I understood the
importance of memory safety, but I couldn't understand why a simple string concatenation required
such cumbersome lifetime annotations. Later I realized, **Rust's designers lived in a different
era.**

Their thought patterns are:

- "Memory safety is a problem that needs to be deliberately solved"
- "Concurrency is a beast that needs careful handling"
- "The type system is a tool for catching errors"

Whereas my thought patterns are:

- "Isn't memory safety something a language should provide by default?"
- "Isn't concurrency as natural as breathing?"
- "Can't the type system become scaffolding for me to explore problems?"

This is not a criticism of Rust. Rust was revolutionary in its time. But **each generation's
"default" is the previous generation's "luxury."**

### 1.2 "Air" and "Obstacles"

Our generation of developers grew up in a world of multi-core CPUs, cloud-native, and mobile
internet. For us:

- Multi-core processors are "air" — we have never experienced the limits of single-core
- Async programming is "air" — we have never experienced synchronous blocking as the default model
- Distributed systems are "air" — we have never experienced a local-first design mindset

When we open a programming language tutorial and see the author spend large portions explaining "why
you need to learn concurrent programming," our inner OS is: **"Isn't this obvious? Why do I need to
learn it?"**

This is the generation gap. **What the previous generation needs to "learn" is "instinct" for our
generation.**

### 1.3 The "Illiteracy" Predicament of the AI Era

When I began engaging with AI programming assistants, I discovered a deeper problem: **existing
languages were never designed with AI in mind.**

- Syntactic ambiguity causes AI hallucinations
- Implicit rules make it impossible for AI to infer behavior
- The fuzzy boundaries of type systems cause AI to give incorrect type suggestions

I have personally seen AI confuse Python's list comprehensions with C++'s lambda expressions, and
mix up Rust's `impl Trait` with TypeScript's generics. This is not AI's fault, **this is a failure
of language design to prepare for the AI era.**

---

## II. Our Programming Instincts — In What Kind of Technical Environment We Grew Up

### 2.1 The Cognitive Patterns of Digital Natives

The programming education trajectory of our generation (born in 2006) is unique:

| Age       | Milestone                                       | Technical Environment                    |
| --------- | ----------------------------------------------- | ---------------------------------------- |
| 9 (2015)  | Scratch/visual programming                      | iPad generation, touch interaction       |
| 12 (2018) | Python/JavaScript                               | Cloud computing rising, Web 2.0 maturing |
| 15 (2021) | Encountered early Copilot                       | AI-assisted programming emerging         |
| 18 (2024) | College entrance exam ended, entered university | GitHub Copilot widely adopted            |
| 19 (2025) | Began designing YaoXiang                        | Claude/GPT-4o era                        |

What does this trajectory mean? **We have native intuition for "human-machine collaborative
programming."**

When we learned to program, AI assistants were already by our side. We have never experienced the
fear of "facing a blank editor alone." We are accustomed to: letting AI generate code skeletons,
then filling in details; letting AI explain syntax we don't understand; letting AI help us debug.

This is not dependency, this is a **symbiotic programming model.**

### 2.2 Concurrency Is Our "Mother Tongue"

I have never experienced an era requiring manual thread pool management. The first time I wrote
concurrent code, I used JavaScript's `async/await`. When I later learned Rust's `async/await`, I was
surprised that such a simple "wait" operation required such a complex `Future` trait, `Pin`, and
`Context`.

**Concurrency is not a feature for us, it is the default state.** Just as multitasking operating
systems are "air" for this generation.

So when YaoXiang adopts the "spawn model," this is not innovation, this is **encoding our instincts
into the language.**

```yaoxiang
# YaoXiang's spawn syntax: concurrency is the default, not explicit
fetch_user(Int) -> User spawn = (id) => { ... }
fetch_posts(User) -> Posts spawn = (user) => { ... }

main() -> Void = () => {
    user = fetch_user(1)     # Automatic parallelism
    posts = fetch_posts(user) # Automatically waits for user, then runs in parallel

    print(posts.title)       # Automatically waits for posts to be ready
}
```

This is not "simplification," this is **restoring our cognitive model.**

### 2.3 A Generation of Visual Thinking

Our generation grew up in Figma, Canva, and Minecraft. We are accustomed to **WYSIWYG (What You See
Is What You Get)** design thinking. When we learn to program, we are puzzled why "writing an
interface" requires crossing so many layers of abstraction.

```yaoxiang
# YaoXiang's visual component syntax
@visual_component
user_profile(User) -> Component = (user) => {
    VStack(spacing=16) {
        Avatar(src=user.avatar, size=64)
        Text(user.name, font="bold 24px")
        Badge(user.role, color="blue")
    }
}
```

This is not just syntactic sugar, this is **acknowledging the thought patterns of our generation.**

---

## III. YaoXiang's Design Response — A Language Designed for the New Generation

### 3.1 Everything Is a Type: A Category-Theoretic Worldview

YaoXiang's core design philosophy is **"everything is a type."** This is not a technical choice, but
a **choice of worldview.**

In YaoXiang's world:

- Values are instances of types
- Types themselves are also instances of types (meta type)
- Functions are mappings from input types to output types
- Modules are combinations of type namespaces

```yaoxiang
# Types as values
MyList = List(Int)    # MyList is now a type value

# Dependent types: types depend on values
type Vector[T, n: Nat] = vector(T, n)

# Pattern matching on types
describe_type(type) -> String = (t) => {
    match t {
        Point(x, y) -> "Point with x=" + x + ", y=" + y
        ok(value) -> "Ok value"
        _ -> "Other type"
    }
}
```

What does this design respond to? It responds to our generation's pursuit of **mathematical
beauty.** The set theory and category theory we encountered while learning mathematics tell us:
**types are the highest-level abstraction.** Why not carry this through to the end?

### 3.2 The Spawn Model: Making Concurrency the Air

YaoXiang's spawn model is a **paradigm disruption of traditional async programming.**

Traditional async programming looks like this:

```rust
// Rust
async fn fetch_data(url: &str) -> Result<Data, Error> {
    let response = reqwest::get(url).await?;
    response.json().await
}
```

You need to understand:

- `async`/`await` syntax
- The `Future` trait
- `Pin` and `Unpin`
- The runtime (tokio/async-std)
- Task scheduler

YaoXiang's spawn model looks like this:

```yaoxiang
# Spawn function: just a spawn marker
fetch_data(String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

# Spawn block: explicit parallelism
compute_all(Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
    (x, y, z) = spawn {
        heavy_calc(a),
        heavy_calc(b),
        another_calc(a, b)
    }
    (x, y, z)
}

# Spawn loop: data parallelism
parallel_sum(Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)
    }
    total
}
```

This is not simplification, this is **redefining the problem.** Traditional async programming asks
"how do we make non-blocking code look like synchronous code?" YaoXiang asks "why should there be a
difference between async and sync?"

**When concurrency becomes air, the syntactic differences disappear.**

### 3.3 AI-Friendly Syntax Design

YaoXiang's design accounts for the needs of AI code generation. This is not a superficial concern of
"AI can understand it," but a deep consideration of "AI participates in the design."

**Design principles:**

1. **Strict structure, unambiguous syntax** — AI won't hallucinate due to syntactic ambiguity
2. **Clear AST, easy location** — AI can precisely locate code positions
3. **Explicit semantics, no hidden behavior** — AI can correctly infer code behavior
4. **Clear code block boundaries** — AI won't misunderstand scope
5. **Complete type information** — AI can give correct type suggestions

```yaoxiang
# Explicit code block boundaries
function_name(Params) -> ReturnType = (params) => {
    # Function body
}

# No optional parentheses (no ambiguity)
foo(T) -> T = (x) => x

# Must use 4-space indentation (clear structure)
if condition {
    do_something()
} else {
    do_other()
}
```

This is not just a style guide, this is **language infrastructure designed for AI collaboration.**

---

## IV. The Generational Thinking Behind Specific Design Decisions

### 4.1 Why Choose "Constructors Are Types"?

YaoXiang's type definitions uniformly use `constructor` syntax. Different variants correspond to
different constructor functions:

```yaoxiang
# Zero-argument constructors (enum style)
type Color = { red: () -> Color, green: () -> Color, blue: () -> Color }

# Multi-argument constructors (struct style)
type Point = Point(x: Float, y: Float)

# Generic constructors
type Result[T, E] = { ok: (T) -> Result[T, E], err: (E) -> Result[T, E] }
```

What does this respond to? It responds to **the type system should be unified, not fragmented.**

In Java, you have `class`, `enum`, `interface`. In Rust, you have `struct`, `enum`, `trait`. In
TypeScript, you have `interface`, `type`, `class`.

Why should types have so many forms? **Types are types; the distinction should be in the form of
values, not in the form of types.**

### 4.2 Why Abandon GC and Adopt the Ownership Model?

YaoXiang adopts a Rust-style ownership model rather than GC.

```yaoxiang
# Default immutable references
process(ref Data) -> Void = (data) => {
    # data is read-only
}

# Mutable references
modify(mut Data) -> Void = (data) => {
    # data can be modified
}

# Transfer ownership
consume(Data) -> Void = (data) => {
    # ownership of data is transferred in
}
```

This is not just a performance choice, it is a **philosophical choice.**

Our generation cares about the environment, cares about resource efficiency. **We do not take
"infinite memory" for granted.** We have cloud service bills, we know every byte has a cost.

At the same time, we don't want to be troubled by GC's "Stop the World" pauses. We are accustomed to
fluid user experiences, to the responsiveness of real-time systems.

The ownership model gives us: **zero-cost abstraction + deterministic performance + memory safety.**

### 4.3 Why Is Currying Core Syntax?

YaoXiang implements syntactic sugar similar to object method calls through currying.

```yaoxiang
# Core function definition
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# Method syntax sugar binding
Point.distance(_) = distance(self, _)

# Calling methods
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)
d1 = distance(p1, p2)     # Direct call
d2 = p1.distance(p2)      # Method syntax
```

What does this respond to? It responds to **we want the purity of functional programming while
preserving the intuitiveness of object orientation.**

Our generation, when learning to program, often started with Python, then encountered JavaScript. We
are accustomed to the `obj.method()` calling style, but we also appreciate the elegance of
functional programming.

Currying makes both **two sides of the same coin.**

---

## V. Beyond Technology — The Cultural Significance of a Generational Perspective

### 5.1 We Need Our Own Voice

Programming language design has long been the discursive domain of "elders." Linus Torvalds started
Linux at 21, and Graydon Hoare was already a senior engineer when designing Rust.

But each generation has its own unique insights. **Young people see problems from different angles,
this is not a flaw, it is value.**

When I designed YaoXiang, I had no historical baggage from C/C++. I didn't need to "adapt" to
existing systems, I could "natively" design new systems.

### 5.2 A New Paradigm of Open Source Collaboration

The open source collaboration our generation understands is:

- Not mailing lists, but Discord communities
- Not official documentation, but interactive tutorials
- Not conference talks, but live coding streams
- Not patent protection, but open collaboration

YaoXiang has been open source from day one. This is not because of idealism, but **this is the way
our generation does things.**

### 5.3 Designed for the AI-Native Era

Current languages are designed for the 2000s (single-core, local, human-written). YaoXiang is
designed for the 2030s (multi-core, distributed, human-AI co-written).

This is not exaggeration, this is an **urgent reality.**

AI is transforming every aspect of programming. Code generation, code review, debugging assistance,
documentation writing — AI is becoming the developer's default partner.

**A language that doesn't consider AI is like a font design that doesn't consider the printer — it
will appear outdated and clumsy.**

---

## VI. Future Outlook — Inviting You to Join

### 6.1 This Is More Than a Project

YaoXiang is not just a programming language project, it is a **generational manifesto.**

It says: our generation is not just learning the tools of our predecessors, we have the ability to
create our own tools. It says: people born in 2006 are not just users of Rust, we can have our own
language.

### 6.2 Seeking Contributors of the "2006 Generation"

I am seeking developers of my age — those who are the first generation of developers to grow up in
the AI era, those who feel "culturally disconnected" from existing languages, those who have their
own design ideas but lack a platform to implement them.

**Your advantages:**

- The same lack of historical baggage
- The same technical intuition
- The same long career horizon

### 6.3 Concrete Next Steps

If you are interested in YaoXiang, you can:

1. **Try using it** — Run your first YaoXiang program
2. **Read the source code** — Understand the implementation of the spawn model
3. **Contribute code** — Implement new features or fix bugs
4. **Participate in design discussions** — Join language design decisions
5. **Spread the philosophy** — Share with more peers

---

## Conclusion: Not Starting Early, But Starting at the Right Time

Rust solved the pain points of the 2000s. YaoXiang can solve the pain points of the 2020s.

This is not a coincidence of history, but an invitation of the times.

**Your greatest asset is not code, but time.**

While peers are still learning to use existing tools, you are creating the next generation of tools.
Ten years from now, when people ask "why was YaoXiang successful," the answer might be:

> "Because it was born in the AI era, designed by the first generation of developers who grew up in
> the AI era — they knew what the future needs, because they are the future."

Begin your era.
