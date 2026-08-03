# We Hid `'a` Inside the Compiler

_— An honest note on the YaoXiang ownership model_

---

How long did it take you, the first time you saw this Rust code, to truly understand it?

```rust
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> &'a str {
        let start = self.pos;
        self.pos += 1;
        &self.input[start..self.pos]
    }
}
```

Three `'a`s. One on the struct, two in the impl block. They all say the same thing: a `Parser`
cannot outlive the `input` it borrows. This is correct. Rust's safety is built on this mechanism.

But when we write this kind of code, there's often a thought: **can't the compiler just figure this
out itself?** Rust's answer is "no"—at least not without changing the borrow model.

**This article isn't "YaoXiang solved this problem." It's "YaoXiang is trying another path—hide `'a`
inside the compiler, let the compiler write it for you. How far we've gotten, and what hard bones
remain unsolved."**

---

## Why Rust Needs `'a`

Rust's `&T` and `&mut T` are pointers—pointers that point at data. Borrowing a value means creating
a reference to it. This reference has its own lifetime. When a reference propagates across function
boundaries (as a return value, stored in a struct), the compiler cannot infer within a single
function how long the reference can live—the programmer needs to use `'a` to provide the information
"this return value and this parameter share a lifetime."

The Rust community didn't stand still. Lifetime elision rules let most simple functions escape
annotation. NLL landed in the 2018 edition, freeing borrows from the constraints of lexical scope.
But when references need to be stored in structs, returned from functions, captured by
closures—**the model itself determines that these scenarios must be annotated by the programmer to
express the relationships between references.**

---

## A Different Angle: Borrows Are Not Pointers, They Are Tokens

YaoXiang's core design is documented in
[RFC-009 (Ownership Model)](/design/rfc/accepted/009-ownership-model). It doesn't change the default
semantics (both are Move), but changes **what a borrow is**.

In YaoXiang, `&T` and `&mut T` **are not pointers**. They are **zero-sized compile-time
tokens**—type-level proof of access permission. Borrowing a value doesn't mean creating a pointer to
it, but creating proof that "I am allowed to access it":

```
&T     →  Guarantees data is immutable. Implements Dup (copyable), multiple read-only tokens can coexist safely
&mut T →  Guarantees exclusive mutability. Does not implement Dup (linear), only one from the same source
```

In Rust you write `&` at the call site (`distance(&p1, &p2)`). In YaoXiang, the compiler sees the
function signature requires `&Point`, and automatically creates a token at the call site—the call
site becomes `distance(p1, p2)`. The cost is that the definer's signature must declare `&`,
otherwise the default Move will consume ownership:

```yaoxiang
# & needed in the signature—compiler sees &Point and auto-creates the token at the call site
check_dimensions: (v: &Vec3) -> Bool = { ... }
check_bounds: (v: &Vec3) -> Bool = { ... }

v = Vec3(1.0, 2.0, 3.0)
if check_dimensions(v) and check_bounds(v) {  # An &Vec3 token is created automatically each time
    # v is still usable
}
```

In Rust you put `&` at the call site; in YaoXiang you put `&` at the definition site. It's not that
annotations disappeared—the location of annotations changed.

---

## The Brand Mechanism: The `'a` Hidden Inside the Compiler

Users never encounter this—but understanding it is necessary to understand what YaoXiang actually
does.

The compiler internally assigns each borrow token a compile-time unique number:

```
User sees              Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)
&mut Point     →  WriteToken(Point, #M)
```

When you access a field from `&Point` to get `&Float`, the latter carries a derived brand:
`#N.field_x`. When you Move a `&mut` token to another variable, the compiler knows the original
variable no longer holds it—this is the foundational capability of Move semantics.

**`#N` is `'a`.** The prefix relationship—`#N` is a prefix of `#N.field_x`—is the outlives
constraint. The same information. The Rust programmer writes `'a`, the YaoXiang compiler writes
`#N`.

The only difference is the inference success rate. Rust's elision rules and NLL cover about 80% of
scenarios. YaoXiang's bet is: **if the language design gives the compiler cleaner input, can it
cover more?**

This bet is supported by several language constraints:

- **No variable shadowing**—`x` has only one identity in this scope, the compiler doesn't need to
  distinguish "which x you mean"
- **Explicit `return`**—what escapes from a block is written out, the compiler doesn't need to infer
  "is the last line the return value"
- **`for` creates a new binding each iteration**—variables across iterations don't interfere, the
  compiler doesn't need to track "what changed in the last iteration"

These are not "conventions." Unlike Java's meaningless getter/setter rituals—each one **turns
information the compiler needs to infer into information already written in the program**. The
compiler doesn't need to guess "which variable do you mean", "did this thing escape", "how do loop
variables change across iterations"—the answer is in the code.

---

## What Brands Can Do—Same as Rust

Because tokens are ordinary types, they obey all the rules of ordinary types. There are no special
prohibitions like "references can't be returned", "references can't be stored in structs", "closures
can't capture references". **But Rust is the same—Rust can do all of it.** The difference is not
capability, but who writes the annotation.

**Returning references—Rust programmer writes `'a`, YaoXiang compiler writes `#N`:**

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()   # The token propagates to the caller, the compiler tracks the brand derivation chain
```

Rust does the same thing, just the programmer needs to write `'a` connecting input and output. In
YaoXiang, the compiler derives automatically through brand paths—`px_ref` (`#N.field_x`) is derived
from `p` (`#N`). The same constraint, a different way of recording it.

**Structs holding references—no lifetime parameter:**

```yaoxiang
Window: Type = {
    target: Point,
    view: &Point,   # Token field, no different from other fields
}
```

In Rust, when a struct has a reference field, `'a` needs to appear in the struct definition and in
all impl blocks—the programmer explicitly annotates the `Window<'a>` lifetime constraint. In
YaoXiang, `view: &Point` doesn't write `'a`, but the brand number still plays the same role inside
the compiler—when the `Window` instance is destroyed, the internal token dies with it. The same
guarantee, different visibility.

**Closure capture—zero-cost Dup token copies:**

```yaoxiang
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)   # The threshold token is copied into the closure, zero overhead
}
```

`&Float` implements Dup (copyable), the closure captures it like capturing a zero-sized integer.
Shares the same rules as automatic borrowing at function calls. Zero user annotation.

---

## The Cost: Annotations Disappear from Signatures

Rust's `'a` has a value often mentioned: it's also documentation.
`fn split_at_mut<'a>(slice: &'a mut [T], mid: usize) -> (&'a mut [T], &'a mut [T])`—`'a` tells the
reader that the two returned slice references come from the same source data.

In reality, the strength of this argument is limited—most Rust beginners don't read `'a` as
documentation, but copy it as a compiler-mandated incantation. But to be fair: in complex borrowing
scenarios, Rust's `'a` at least gives a starting point for tracing data flow. In YaoXiang, you need
to understand the brand derivation chain—brands are invisible to users, which depends on the
toolchain, and the toolchain is not yet mature.

---

## Token Conflict Detection: The Same Proof Pipeline

Rust has a separate "borrow checker." YaoXiang's **design direction** is to unify borrow conflicts
into the type-checking proof pipeline
([RFC-027 (Compile-time Predicates and Unified Static Verification)](/design/rfc/accepted/027-compile-time-evaluation-types)).

A token conflict is a Hoare proposition:

```
{ conflicting ReadToken is dead } data.push(4) { WriteToken safely obtained }
```

```yaoxiang
# &mut T is a linear type—after Move the original variable no longer holds it
bad: (p: &mut Point) -> Void = {
    p2: &mut Point = p    # The WriteToken transfers from p to p2
    p.x = 10.0            # { p holds WriteToken } p.x = 10.0 { safe }
}                          # → p's WriteToken has been Moved → Disproved

# &T is Dup—copyable
good: (p: &Point) -> Void = {
    p2: &Point = p        # Copy the read-only token
    print(p.x)            # OK, two read-only tokens coexist
}
```

Shares the same error reporting path as type errors and predicate verification failures. You don't
need to learn two diagnostic systems. **But the cost is:** a complex token conflict in Rust produces
a carefully worded borrow check error; in YaoXiang it might manifest as "WriteToken(#7.field_x)
conflicts with WriteToken(#7)"—technically accurate, but brand numbers are meaningless to human
readers. The interpretability of error messages is an unvalidated area.

---

## The `ref` Keyword: Automatically Choosing Rc/Arc

Tokens cannot cross tasks (cross-thread)—they are compile-time proofs, not runtime values. For
cross-scope sharing, use `ref`:

```yaoxiang
shared_data = ref Point(1.0, 2.0)   # The compiler's escape analysis automatically picks Rc or Arc

spawn {
    print(shared_data.x)   # Crosses into the task → compiler picks Arc
}
```

- Doesn't escape into a spawn block → `Rc` (non-atomic reference counting)
- Escapes into a spawn block → `Arc` (atomic reference counting)

The cost: when reading code, you cannot tell from the local context whether `ref` is Rc or Arc. A
single refactor (wrapping code in spawn) might silently change the reference counting
implementation—you won't get a compiler reminder. Performance changes are implicit.

---

## The Current Hard Bone: RAII Is Too Crude

Earlier we said tokens are values, and lifetimes are managed by RAII. But the RAII rules for
ordinary values are: **a value lives until the end of the scope.** This is precisely Rust's pre-NLL
problem—borrows last until the end of the block, even if you've long stopped using them.

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_view: &Header = data.header()    # Derive &Header from &mut Data
    header_info = parse_header(header_view) # ← last use of header_view
    # header_view doesn't need to live until the function ends—
    # but RAII keeps it alive until }

    data.modify(header_info)   # ❌ ReadToken is still "alive", WriteToken is blocked
}
```

Rust's NLL analyzes last use rather than lexical scope. YaoXiang needs the same capability. The
current approach is to connect token liveness analysis to the proof pipeline as well—three layers:

1. **Fast path**—reuse the existing BorrowChecker (linear scan, IR instruction-level). Scenarios
   where tokens are fully used within a single basic block pass directly
2. **Structural analysis**—brand tree prefix matching (judging who conflicts with whom) + DAG
   consumer queries (judging whether the last consumer of a token is after the current node)
3. **SMT solving**—only activated when loop conditions and other things need logical reasoning

The proof pipeline infrastructure (the `Proved/Disproved/Unproven` three-valued return, the Z3 SMT
backend, the assumption stack) has been partially implemented in
`src/frontend/core/typecheck/proof/`. The ownership layer (`layers/ownership.rs`) is still a
skeleton—it directly returns Proved without doing actual checks. It's being filled in.

The current version's solution is to manually nest blocks to shorten token scope:

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_info = {
        header_view: &Header = data.header()
        parse_header(header_view)
    }   # header_view is released when the block ends
    data.modify(header_info)   # ✅
}
```

This is real friction. Everyone coming from Rust will encounter it. Whether it can be
eliminated—depends on the effect of connecting the pipeline to the ownership layer.

---

## The Hardest Question: Fallback

Rust's `'a` is not only a burden—it's also a fallback. The compiler can't infer, the programmer
annotates lifetime relationships, the compiler verifies. **You have a pen.**

YaoXiang's fallback theoretically should be a compile-time **proof function** (RFC-027 §4.2):
compiler auto-inference fails → the programmer writes a function, the return type is the proposition
"tokens don't conflict" → the compiler verifies the function's type. But—

What does a "tokens don't conflict" proof function look like? How does a user construct a value of
type `WriteTokenAvailable`? Do they need to understand the prefix relationship between brand numbers
`#N` and `#N.field_x`?

**If proof functions require users to understand brand numbers—then we've just renamed `'a` to `#1`.
No savings at all.**

This question has no answer yet. This is the place where the whole experiment is most likely to get
stuck.

---

## Nine Iterations of RFC-009

This wasn't a design dreamed up in an ivory tower. RFC-009 went through nine major versions:

| Version | Key Changes                                                                                                                 | Why it was overturned                                                        |
| ------- | --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| v1–v7   | Based on Rust's ownership model, gradually adding consumption analysis, inverse functions, field-level mutability           | Over-engineered, complexity out of control                                   |
| **v8**  | "Beggar-version borrow"—`&T`/`&mut T` can only be parameters, not returned, not stored in structs, not captured by closures | Three hardcoded prohibitions. Expressiveness severely limited                |
| **v9**  | Borrow token system—`&T`/`&mut T` are ordinary types, obey ordinary rules                                                   | Eliminated special rules, but pushed brand tracking down inside the compiler |

The v8 to v9 transition was the real breakthrough: from three prohibitions to zero special rules.
But what v9 eliminated is the user-visible rules, not the system's intrinsic complexity—brand
mechanism, derivation tracking, same-source conflict detection, these still exist, just moved inside
the compiler. Unifying borrow checking into the proof pipeline is one direction, but whether it can
run on real code, how the fallback should be designed—is still being verified.

---

## Final Words

We didn't eliminate `'a`. `#1` is `'a`—the same information, in a different location.

The experiment's bet is: language design constraints (no shadowing, explicit return, `for` new
bindings, `{}` DAG semantics) give the compiler cleaner input, and brand derivation might succeed
automatically in most scenarios where Rust's lifetime elision rules fail. If so—users no longer need
to write `'a`, don't need to distinguish annotation from elision, don't need to learn the borrow
checker. If not—or if the fallback mechanism (proof functions) requires users to understand brand
numbers—then it's just a reinvention.

We're working on it. We'll write again when we have results.

---

_YaoXiang is a programming language under development. See
[RFC-009](/design/rfc/accepted/009-ownership-model) for the ownership model,
[RFC-023](/design/rfc/accepted/023-closure-capture-model) for closure capture,
[RFC-024](/design/rfc/accepted/024-concurrency-model) for the concurrency model._
