# We hid `'a` inside the compiler

_—An honest note on the YaoXiang ownership model_

---

How long did it take you, the first time you saw this Rust code, to actually understand it?

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

Three `'a`. One on the struct, two in the impl block. They all say the same thing: `Parser` cannot
outlive the `input` it borrows. This is correct. Rust's safety is built on this mechanism.

But when we write code like this, there's often a thought: **can't the compiler figure this out on
its own?** Rust's answer is "no"—at least not without changing the borrow model.

**This post isn't "YaoXiang solved this problem." It's "YaoXiang is trying another path—hiding `'a`
inside the compiler, letting it write the annotation for you. How far it's gotten, and what hard
problems remain unsolved."**

---

## Why Rust needs `'a`

Rust's `&T` and `&mut T` are pointers—pointers that point at data. Borrowing a value means creating
a reference to it. This reference has its own lifetime. When the reference propagates across
function boundaries (as a return value, stored in a struct), the compiler cannot infer within a
single function how long the reference can live—the programmer must use `'a` to provide the
information that "this return value and this parameter share a lifetime."

The Rust community hasn't stood still. Lifetime elision rules let most simple functions skip
annotations. NLL landed in 2018, freeing borrows from lexical scope. But when a reference needs to
be stored in a struct, returned from a function, or captured by a closure—**the model itself
determines that these scenarios must have the programmer annotate relationships between
references.**

---

## A different angle: a borrow isn't a pointer, it's a token

YaoXiang's core design is documented in
[RFC-009 (Ownership Model)](../design/rfc/accepted/009-ownership-model.md). It didn't change the
default semantics (still Move), but it changed **what a borrow fundamentally is**.

In YaoXiang, `&T` and `&mut T` are **not pointers**. They are **zero-sized compile-time
tokens**—type-level proofs of access permission. Borrowing a value isn't creating a pointer to it;
it's creating proof that "I am allowed to access it":

```
&T     →  Guarantees data is immutable. Implements Dup (copyable); multiple read-only tokens can safely coexist
&mut T →  Guarantees exclusive mutability. Does not implement Dup (linear); only one can exist from the same source
```

In Rust, you write `&` at the call site (`distance(&p1, &p2)`). In YaoXiang, the compiler sees the
function signature requires `&Point` and automatically creates the token at the call site—the call
site becomes `distance(p1, p2)`. The price is that the definer's signature must declare `&`,
otherwise the default Move will consume ownership:

```yaoxiang
# The signature needs &—the compiler sees &Point and automatically creates a token at the call
check_dimensions: (v: &Vec3) -> Bool = { ... }
check_bounds: (v: &Vec3) -> Bool = { ... }

v = Vec3(1.0, 2.0, 3.0)
if check_dimensions(v) and check_bounds(v) {  # A &Vec3 token is created each time
    # v is still available
}
```

In Rust you put `&` at the call site; in YaoXiang you put `&` at the definition site. The annotation
didn't disappear—it moved.

---

## The brand mechanism: `'a` hidden inside the compiler

Users never touch this—but understanding it is necessary to understand what YaoXiang actually did.

The compiler internally assigns a unique compile-time number to each borrow token:

```
What the user sees     What the compiler sees internally
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)
&mut Point     →  WriteToken(Point, #M)
```

When you access a field from `&Point` to get `&Float`, the latter carries a derived brand:
`#N.field_x`. When you Move an `&mut` token to another variable, the compiler knows the original
variable no longer holds it—this is the basic capability underpinning Move semantics.

**`#N` is `'a`.** The prefix relation—`#N` is the prefix of `#N.field_x`—is the outlives constraint.
The same information. Rust's programmers write `'a`; YaoXiang's compiler writes `#N`.

The only difference is inference success rate. Rust's elision rules and NLL cover about 80% of
scenarios. YaoXiang's bet is: **if the language design gives the compiler cleaner input, can it
cover more?**

This bet is supported by several language constraints:

- **No variable shadowing**—`x` has only one identity in its scope; the compiler doesn't need to
  distinguish "which x are you referring to"
- **Explicit `return`**—what escapes a block is written out; the compiler doesn't need to infer "is
  the last line a return value"
- **`for` creates a new binding per iteration**—variables across iterations don't interfere; the
  compiler doesn't need to track "what did the last iteration change"

These aren't "rituals." Unlike Java's meaningless getter/setter ceremonies—each one **turns
information the compiler would need to infer into information already written in the program.** The
compiler doesn't need to guess "which variable you mean," "whether this thing escaped," "how loop
variables change across iterations"—the answers are in the code.

---

## What brands can do—same as Rust

Because tokens are ordinary types, they obey all the rules of ordinary types. No special
prohibitions like "references can't be returned," "references can't be stored in structs," or
"closures can't capture references." **But Rust does the same thing—Rust can do all of it.** The
difference isn't capability; it's who writes the annotation.

**Returning references—Rust programmers write `'a`, YaoXiang's compiler writes `#N`:**

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()   # Token propagates to caller; compiler tracks brand derivation chain
```

Rust does the same thing; only the programmer needs to write `'a` to connect input and output. In
YaoXiang, the compiler derives it automatically through brand paths—`px_ref` (`#N.field_x`) derives
from `p` (`#N`). The same constraint, different way of recording it.

**Structs holding references—no lifetime parameters:**

```yaoxiang
Window: Type = {
    target: Point,
    view: &Point,   # Token field, no different from other fields
}
```

In Rust, when a struct has a reference field, `'a` needs to appear in the struct definition and in
every impl block—the programmer explicitly annotates `Window<'a>`'s lifetime constraint. In
YaoXiang, `view: &Point` doesn't write `'a`, but the brand number still plays the same role inside
the compiler—when a `Window` instance is destroyed, the token inside it dies with it. The same
guarantee, different visibility.

**Closure capture—zero-cost Dup token copy:**

```yaoxiang
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)   # threshold token copied into the closure, zero overhead
}
```

`&Float` implements Dup (copyable); the closure captures it like capturing a zero-sized integer. It
shares the same rule as the automatic borrow for function calls. Zero annotations from the user.

---

## The cost: annotations vanish from signatures

Rust's `'a` has a frequently cited value: it also serves as documentation.
`fn split_at_mut<'a>(slice: &'a mut [T], mid: usize) -> (&'a mut [T], &'a mut [T])`—`'a` tells the
reader that the two returned slices reference the same original data.

In practice, this argument has limited force—most Rust beginners don't read `'a` as documentation;
they copy it as a compiler-mandated incantation. But to be fair: in complex borrow scenarios, Rust's
`'a` at least gives a starting point for tracing data flow. In YaoXiang you need to understand the
brand derivation chain—brands are invisible to the user, which depends on tooling, and the tooling
isn't mature yet.

---

## Token conflict detection: the same proof pipeline

Rust has a separate "borrow checker." YaoXiang's **design direction** is to unify borrow conflicts
into the type-checker's proof pipeline
([RFC-027 (Compile-time Predicates and Unified Static Verification)](../design/rfc/accepted/027-compile-time-evaluation-types.md)).

A token conflict is a Hoare proposition:

```
{ Conflicting ReadToken is dead } data.push(4) { WriteToken safely acquired }
```

```yaoxiang
# &mut T is a linear type—after Move, the original variable no longer holds it
bad: (p: &mut Point) -> Void = {
    p2: &mut Point = p    # WriteToken transfers from p to p2
    p.x = 10.0            # { p holds WriteToken } p.x = 10.0 { safe }
}                          # → p's WriteToken has been Moved → Disproved

# &T is Dup—copyable
good: (p: &Point) -> Void = {
    p2: &Point = p        # Copy the read-only token
    print(p.x)            # OK, two read-only tokens coexist
}
```

It shares the same error-reporting path as type errors and predicate verification failures. You
don't need to learn two diagnostic systems. **But the cost is:** a complex token conflict in Rust
produces a carefully worded borrow-checker error; in YaoXiang it might surface as
"WriteToken(#7.field_x) conflicts with WriteToken(#7)"—technically accurate, but brand numbers are
meaningless to human readers. Error-message interpretability is an unverified territory.

---

## The `ref` keyword: automatic Rc/Arc selection

Tokens cannot cross tasks (cross threads)—they are compile-time proofs, not runtime values. For
cross-scope sharing, use `ref`:

```yaoxiang
shared_data = ref Point(1.0, 2.0)   # The compiler's escape analysis automatically picks Rc or Arc

spawn {
    print(shared_data.x)   # Crosses a task → compiler picks Arc
}
```

- Doesn't escape into a spawn block → `Rc` (non-atomic reference counting)
- Escapes into a spawn block → `Arc` (atomic reference counting)

The cost: when reading code, you can't tell locally whether `ref` is Rc or Arc. A single refactor
(wrapping code in spawn) can silently change the reference-counting implementation—you won't get a
compiler heads-up. Performance changes are implicit.

---

## The current hard problem: RAII is too coarse

Earlier we said tokens are values, with lifetimes managed by RAII. But the RAII rule for ordinary
values is: **a value lives until the end of its scope.** This is exactly the problem Rust had before
NLL—borrows lasted until the end of the entire block, even if you stopped using them long before.

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_view: &Header = data.header()    # Derive &Header from &mut Data
    header_info = parse_header(header_view) # ← Last use of header_view
    # header_view doesn't need to live until the end of the function—
    # but RAII keeps it alive until }

    data.modify(header_info)   # ❌ ReadToken is still "alive"; WriteToken is blocked
}
```

Rust's NLL analyzes last-use rather than lexical scope. YaoXiang needs the same capability. The plan
in progress is to plug token liveness analysis into the proof pipeline—three layers:

1. **Fast path**—reuse the existing BorrowChecker (linear scan, IR instruction level). Scenarios
   where a token is consumed within a single basic block pass directly
2. **Structural analysis**—brand-tree prefix matching (judging who conflicts with whom) + DAG
   consumer queries (judging whether the last consumer of a token is after the current node)
3. **SMT solving**—activates only when logical reasoning is needed, e.g., loop conditions

The proof pipeline infrastructure (three-valued return of `Proved/Disproved/Unproven`, Z3 SMT
backend, assumption stack) is partially implemented in `src/frontend/core/typecheck/proof/`. The
ownership layer (`layers/ownership.rs`) is still a skeleton—it returns Proved directly without doing
any actual check. Being filled in.

The current version's workaround is manually nesting blocks to shorten token scope:

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_info = {
        header_view: &Header = data.header()
        parse_header(header_view)
    }   # header_view is released at block end
    data.modify(header_info)   # ✅
}
```

This is real friction. Everyone coming from Rust will hit it. Whether it can be eliminated—depends
on how effectively the pipeline plugs into the ownership layer.

---

## The hardest problem: the escape hatch

Rust's `'a` isn't just a burden—it's also an escape hatch. The compiler can't infer it; the
programmer annotates the lifetime relationship; the compiler verifies. **You have a pen.**

YaoXiang's escape hatch is theoretically supposed to be a compile-time **proof function** (RFC-027
§4.2): the compiler's automatic derivation fails → the programmer writes a function whose return
type is the proposition "tokens don't conflict" → the compiler type-checks the function. But—

What does a "tokens don't conflict" proof function look like? How does a user construct a value of
type `WriteTokenAvailable`? Do they need to understand the prefix relationship between brand numbers
`#N` and `#N.field_x`?

**If the proof function requires the user to understand brand numbers—then we've just renamed `'a`
to `#1`. Nothing saved.**

This question has no answer yet. This is where the experiment is most likely to die.

---

## Nine iterations of RFC-009

This design wasn't cooked up in an ivory tower. RFC-009 went through nine major versions:

| Version | Key Change                                                                                                               | Why it was overturned                                                                  |
| ------- | ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| v1–v7   | Based on Rust's ownership model, gradually added consumption analysis, inverse functions, field-level mutability         | Over-engineered; complexity ran out of control                                         |
| **v8**  | "Beggar-version borrow"—`&T`/`&mut T` could only be parameters, not returned, stored in structs, or captured by closures | Three hard-coded prohibitions. Severely limited expressiveness                         |
| **v9**  | Borrow token system—`&T`/`&mut T` are ordinary types obeying ordinary rules                                              | Eliminated special rules, but pushed brand tracking down into the compiler's internals |

The jump from v8 to v9 is the real breakthrough: from three prohibitions to zero special rules. But
what v9 eliminated was user-visible rules, not the system's intrinsic complexity—brand mechanisms,
derivation tracking, same-source conflict detection, these all still exist, just inside the
compiler. Unifying borrow checking into the proof pipeline is a direction, but whether it works on
real code and how the escape hatch is designed—still being verified.

---

## Closing note

We didn't eliminate `'a`. `#1` is `'a`—the same information, in a different place.

The experiment's bet is: language design constraints (no shadowing, explicit return, fresh `for`
bindings, `{}` DAG semantics) give the compiler cleaner input, and brand derivation might
automatically succeed in most of the scenarios where Rust's lifetime elision rules fail. If it
can—users no longer need to write `'a`, no longer need to distinguish annotation from elision, no
longer need to learn the borrow checker. If it can't—or if the escape hatch (proof functions)
requires the user to understand brand numbers—then it's just a reinvention.

In progress. Will write again when there's a result.

---

_YaoXiang is a programming language under active development. The ownership model is in
[RFC-009](../design/rfc/accepted/009-ownership-model.md), closure capture in
[RFC-023](../design/rfc/deprecated/023-closure-capture-model.md), and the concurrency model in
[RFC-024](../design/rfc/accepted/024-concurrency-model.md)._
