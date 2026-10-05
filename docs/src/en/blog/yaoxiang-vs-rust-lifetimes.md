# We hid `'a` inside the compiler

_— An honest note on the YaoXiang ownership model_

---

How long did it take you to truly understand the first time you saw this Rust code?

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

Three `'a`s. One on the struct, two in the impl block. They all say the same thing: `Parser` cannot
outlive the `input` it borrows. This is correct. Rust's safety is built on this mechanism.

But when we write code like this, we often have a thought: **Can't the compiler handle this
itself?** Rust's answer is "no"—at least not without changing the borrow model.

**This article is not "YaoXiang solved this problem." It is "YaoXiang is trying another path—hiding
`'a` inside the compiler, letting the compiler write it for you. How far we've gotten, and what hard
nuts remain unsolved."**

---

## Why Rust needs `'a`

Rust's `&T` and `&mut T` are pointers—pointers to data. Borrowing a value means creating a reference
to it. This reference has its own lifetime. When references propagate across function boundaries (as
return values, stored in structs), the compiler cannot infer within a single function how long a
reference can live—it needs the programmer to use `'a` to provide information that "this return
value and this parameter share a lifetime."

The Rust community hasn't stood still. Lifetime elision rules free most simple functions from
annotations. NLL landed in the 2018 edition, freeing borrows from lexical scope. But when references
need to be stored in structs, returned from functions, captured by closures—**the model itself
determines that these scenarios must be annotated by the programmer with relationships between
references.**

---

## A different angle: borrows are not pointers, they are tokens

The core design of YaoXiang is documented in
[RFC-009 (Ownership Model)](../rfc/accepted/009-ownership-model.md). It didn't change the default
semantics (still Move), but changed **the ontology of borrowing**.

In YaoXiang, `&T` and `&mut T` are **not pointers**. They are **zero-sized compile-time
tokens**—type-level proof of access permission. Borrowing a value doesn't create a pointer to it,
but a proof of "I am allowed to access it":

```
&T     →  Guarantees data is immutable. Implements Dup (copyable); multiple read-only tokens coexist safely
&mut T →  Guarantees exclusive mutability. Does not implement Dup (linear); only one of the same origin can exist
```

In Rust you write `&` at the call site (`distance(&p1, &p2)`). In YaoXiang, the compiler sees the
function signature requires `&Point` and automatically creates the token at the call site—the call
site becomes `distance(p1, p2)`. The price is that the definer's signature must declare `&`,
otherwise the default Move would consume ownership:

```yaoxiang
# The signature needs &—the compiler sees &Point and automatically creates a token at the call site
check_dimensions: (v: &Vec3) -> Bool = { ... }
check_bounds: (v: &Vec3) -> Bool = { ... }

v = Vec3(1.0, 2.0, 3.0)
if check_dimensions(v) and check_bounds(v) {  # An &Vec3 token is created automatically each time
    # v is still usable
}
```

In Rust you put `&` at the call site; in YaoXiang you put `&` at the definition site. The annotation
didn't disappear—its location changed.

---

## The brand mechanism: `'a` hidden inside the compiler

Users never touch this—but understanding it is necessary to understand what YaoXiang actually does.

The compiler internally assigns a unique compile-time identifier to each borrow token:

```
What the user sees        The compiler's internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)
&mut Point     →  WriteToken(Point, #M)
```

When you access a field from `&Point` to get `&Float`, the latter carries a derived brand:
`#N.field_x`. When you Move an `&mut` token to another variable, the compiler knows the original
variable no longer holds it—this is the basic capability underlying Move semantics.

**`#N` is `'a`.** The prefix relationship—`#N` is a prefix of `#N.field_x`—is the outlives
constraint. The same information. The Rust programmer writes `'a`; the YaoXiang compiler writes
`#N`.

The only difference is the inference success rate. Rust's elision rules and NLL cover about 80% of
scenarios. YaoXiang's bet is: **If the language design gives the compiler cleaner input, can it
cover more?**

This bet is supported by several language constraints:

- **No variable shadowing**—`x` has only one identity in this scope, so the compiler doesn't need to
  distinguish "which x do you mean"
- **Explicit `return`**—what escapes from a block is written down, so the compiler doesn't need to
  infer "is the last line the return value"
- **`for` creates a new binding each iteration**—variables across iterations don't interfere, so the
  compiler doesn't need to track "what changed in the last iteration"

These aren't "conventions." Unlike Java's meaningless getter/setter rituals—each one **turns
information the compiler needs to infer into information already written in the program**. The
compiler doesn't need to guess "which variable do you mean," "has this thing escaped," "how does the
loop variable change across iterations"—the answers are in the code.

---

## What brands can do—same as Rust

Because tokens are ordinary types, they obey all the rules of ordinary types. No special
prohibitions like "references can't be returned," "references can't be stored in structs," "closures
can't capture references." **But Rust can do the same—Rust can do all of it.** The difference isn't
capability; it's who writes the annotation.

**Returning references—Rust programmers write `'a`, the YaoXiang compiler writes `#N`:**

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()   # Token propagates to the caller; the compiler tracks the brand derivation chain
```

Rust does the same thing; the programmer just needs to write `'a` to connect input and output. In
YaoXiang, the compiler derives automatically through the brand path—`px_ref` (`#N.field_x`) is
derived from `p` (`#N`). The same constraint, a different way of recording it.

**Structs holding references—no lifetime parameters:**

```yaoxiang
Window: Type = {
    target: Point,
    view: &Point,   # Token field, no different from other fields
}
```

In Rust, when a struct has a reference field, `'a` needs to appear in the struct definition and all
impl blocks—the programmer explicitly annotates `Window<'a>`'s lifetime constraint. In YaoXiang,
`view: &Point` doesn't write `'a`, but the brand identifier still plays the same role inside the
compiler—when a `Window` instance is destroyed, the token inside dies with it. The same guarantee,
different visibility.

**Closure capture—zero-cost copy of Dup tokens:**

```yaoxiang
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)   # threshold token is copied into the closure, zero overhead
}
```

`&Float` implements Dup (copyable); the closure captures it like capturing a zero-sized integer.
Same set of rules as the automatic borrowing for function calls. Zero annotation from the user.

---

## The cost: annotations disappear from signatures

Rust's `'a` has a frequently cited value: it is also documentation.
`fn split_at_mut<'a>(slice: &'a mut [T], mid: usize) -> (&'a mut [T], &'a mut [T])`—`'a` tells the
reader that the two returned slice references come from the same original data.

In practice the strength of this argument is limited—most Rust beginners don't read `'a` as
documentation, but copy it as a compiler-mandated incantation. But to be fair: in complex borrowing
scenarios, Rust's `'a` at least provides a starting point for tracing data flow. In YaoXiang, you
need to understand the brand derivation chain—brands are invisible to users; this depends on
tooling, and the tooling isn't mature yet.

---

## Token conflict detection: the same proof pipeline

Rust has an independent "borrow checker." YaoXiang's **design direction** is to unify borrow
conflicts into the type checker's proof pipeline
([RFC-027 (Compile-Time Predicates and Unified Static Verification)](../rfc/accepted/027-compile-time-evaluation-types.md)).

Token conflict is a Hoare proposition:

```
{ Conflicting ReadToken is dead } data.push(4) { WriteToken safely acquired }
```

```yaoxiang
# &mut T is a linear type—after Move, the original variable no longer holds it
bad: (p: &mut Point) -> Void = {
    p2: &mut Point = p    # WriteToken transferred from p to p2
    p.x = 10.0            # { p holds WriteToken } p.x = 10.0 { safe }
}                          # → p's WriteToken has been Moved → Disproved

# &T is Dup—copyable
good: (p: &Point) -> Void = {
    p2: &Point = p        # Copy the read-only token
    print(p.x)            # OK, two read-only tokens coexist
}
```

Shared with type errors and predicate verification failures in the same error reporting path. You
don't need to learn two diagnostic systems. **But the cost is:** a complex token conflict in Rust
produces a carefully worded borrow check error; in YaoXiang it might appear as
"WriteToken(#7.field_x) conflicts with WriteToken(#7)"—technically accurate, but brand identifiers
are meaningless to human readers. The interpretability of error messages is an unvalidated
territory.

---

## The `ref` keyword: automatically choose Rc/Arc

Tokens cannot cross tasks (cross-thread)—they are compile-time proofs, not runtime values. Use `ref`
for cross-scope sharing:

```yaoxiang
shared_data = ref Point(1.0, 2.0)   # Compiler escape analysis automatically picks Rc or Arc

spawn {
    print(shared_data.x)   # Cross-task → compiler picks Arc
}
```

- Doesn't escape into the spawn block → `Rc` (non-atomic reference counting)
- Escapes into the spawn block → `Arc` (atomic reference counting)

Cost: when reading code, you can't tell from the local context whether `ref` is Rc or Arc. A single
refactor (wrapping code in spawn) might quietly change the reference counting implementation—you
won't get a compiler reminder. The performance change is implicit.

---

## The current hard nut: RAII is too coarse

Earlier I said tokens are values, and lifetimes are managed by RAII. But the RAII rules for ordinary
values are: **a value lives until the end of its scope.** This is exactly the problem Rust had
before NLL—borrows lasted until the end of the block, even after you stopped using them.

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_view: &Header = data.header()    # Derive &Header from &mut Data
    header_info = parse_header(header_view) # ← Last use of header_view
    # header_view doesn't need to live until the end of the function—
    # but RAII keeps it alive until }

    data.modify(header_info)   # ❌ ReadToken is still "alive", WriteToken is blocked
}
```

Rust's NLL analyzes the last use rather than the lexical scope. YaoXiang needs the same capability.
The approach currently being worked on is hooking token liveness analysis into the proof
pipeline—three layers:

1. **Fast path**—reuse the existing BorrowChecker (linear scan, IR instruction-level). Scenarios
   where a token is used up within the same basic block pass directly
2. **Structural analysis**—brand tree prefix matching (judging who conflicts with whom) + DAG
   consumer queries (judging whether the last consumer of a token is after the current node)
3. **SMT solving**—activated only when logical reasoning is needed, e.g., loop conditions

The proof pipeline infrastructure (`Proved/Disproved/Unproven` three-valued return, Z3 SMT backend,
assumption stack) is partially implemented in `src/frontend/core/typecheck/proof/`. The ownership
layer (`layers/ownership.rs`) is still a skeleton—returns Proved directly without doing actual
checks. Being filled in.

The current version's workaround is to manually nest blocks to shorten the token's scope:

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_info = {
        header_view: &Header = data.header()
        parse_header(header_view)
    }   # header_view released when block ends
    data.modify(header_info)   # ✅
}
```

This is real friction. Everyone coming from Rust will hit it. Whether it can be eliminated—depends
on the effect of hooking the pipeline into the ownership layer.

---

## The hardest problem: fallback

Rust's `'a` is not just a burden—it is also a fallback. The compiler can't infer it, the programmer
annotates the lifetime relationship, the compiler verifies. **You have a pen.**

YaoXiang's fallback theoretically should be a compile-time **proof function** (RFC-027 §4.2):
automatic inference by the compiler fails → the programmer writes a function whose return type is
the proposition "tokens don't conflict" → the compiler verifies this function's type. But—

What does a proof function for "tokens don't conflict" look like? How does a user construct a value
of type `WriteTokenAvailable`? Do they need to understand the prefix relationship between brand
identifiers `#N` and `#N.field_x`?

**If proof functions require users to understand brand identifiers—then we've just renamed `'a` to
`#1`. We haven't saved anything.**

This question has no answer yet. This is where the whole experiment is most likely to die.

---

## RFC-009's nine iterations

This design didn't come out of an ivory tower. RFC-009 went through nine major versions:

| Version | Key Change                                                                                                                   | Why Rejected                                                                           |
| ------- | ---------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| v1–v7   | Based on Rust's ownership model, gradually adding consumption analysis, inverse functions, field-level mutability            | Over-engineered, complexity out of control                                             |
| **v8**  | "Beggar-version borrow"—`&T`/`&mut T` can only be parameters, cannot be returned, stored in structs, or captured by closures | Three hard-coded prohibitions. Expressiveness severely limited                         |
| **v9**  | Borrow token system—`&T`/`&mut T` are ordinary types that obey ordinary rules                                                | Eliminates special rules, but pushes brand tracking down into the compiler's internals |

The jump from v8 to v9 is the real breakthrough: from three prohibitions to zero special rules. But
v9 eliminates the user-visible rules, not the system's inherent complexity—the brand mechanism,
derivation tracking, same-origin conflict detection still exist, just moved inside the compiler.
Unifying borrow checking into the proof pipeline is one direction, but whether it can run on real
code and how the fallback is designed—still being validated.

---

## Final words

We didn't eliminate `'a`. `#1` is `'a`—the same information, in a different location.

The experiment's bet is: language design constraints (no shadowing, explicit return, `for` new
binding, `{}` DAG semantics) give the compiler cleaner input, and brand derivation may automatically
succeed in most scenarios where Rust's lifetime elision rules fail. If it can—users no longer need
to write `'a`, no longer need to distinguish between annotated and elided, no longer need to learn
the borrow checker. If it can't—or if the fallback mechanism (proof functions) requires users to
understand brand identifiers—then it's just a reinvention.

Working on it. Will write again when there are results.

---

_YaoXiang is a programming language under development. For the ownership model, see
[RFC-009](../rfc/accepted/009-ownership-model.md); for closure capture, see
[RFC-023](../rfc/deprecated/023-closure-capture-model.md); for the concurrency model, see
[RFC-024](../rfc/accepted/024-concurrency-model.md)._
