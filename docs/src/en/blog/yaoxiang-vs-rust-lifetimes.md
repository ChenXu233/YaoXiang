# We Hid `'a` Inside the Compiler

_—An honest note on YaoXiang's ownership model_

---

How long did it take you to really understand this Rust code when you first saw it?

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

Three `'a`s. One on the struct, two in the impl block. They're all saying the same thing: `Parser`
cannot outlive the `input` it borrows. This is correct. Rust's safety is built on this mechanism.

But when we write code like this, there's often a thought: **Couldn't the compiler figure this out
itself?** Rust's answer is "no"—at least not without changing the borrowing model.

**This article is not "YaoXiang solved this problem." It's "YaoXiang is trying another path—hiding
`'a` inside the compiler, letting the compiler write it for you. Where we've gotten, and what the
hard problems are that remain unsolved."**

---

## Why Rust Needs `'a`

Rust's `&T` and `&mut T` are pointers—pointers to data. Borrowing a value means creating a reference
to it. This reference has its own lifetime. When a reference propagates across function boundaries
(as a return value, stored in a struct), the compiler cannot infer how long the reference will live
within a single function—it needs the programmer to provide "this return value and this parameter
share a lifetime" information via `'a`.

The Rust community hasn't stood still. Lifetime elision rules let most simple functions skip
annotations. NLL landed in the 2018 edition, freeing borrows from lexical scope. But when references
need to be stored in structs, returned from functions, or captured by closures—**the model itself
determines that the programmer must annotate the relationships between references.**

---

## A Different Perspective: Borrow Is Not a Pointer, It's a Token

YaoXiang's core design is documented in
[RFC-009 (Ownership Model)](/design/rfc/accepted/009-ownership-model). It doesn't change the default
semantics (all Move), but it changes **what a borrow actually is**.

In YaoXiang, `&T` and `&mut T` **are not pointers**. They are **zero-sized compile-time
tokens**—type-level proof of access permission. Borrowing a value doesn't create a pointer to it; it
creates proof that "I am allowed to access it":

```
&T     →  Guarantees immutable data. Implements Dup (copyable), multiple read-only tokens coexist safely
&mut T →  Guarantees exclusive mutability. Does not implement Dup (linear), only one can exist from the same source
```

In Rust you write `&` at the call site (`distance(&p1, &p2)`). In YaoXiang, the compiler sees the
function signature requires `&Point`, automatically creates the token at the call site—the call site
becomes `distance(p1, p2)`. The cost is that the definer's signature must declare `&`, otherwise the
default Move semantics will consume ownership:

```yaoxiang
# & is needed in the signature—the compiler sees &Point, auto-creates token at call site
check_dimensions: (v: &Vec3) -> Bool = { ... }
check_bounds: (v: &Vec3) -> Bool = { ... }

v = Vec3(1.0, 2.0, 3.0)
if check_dimensions(v) && check_bounds(v) {  # &Vec3 token auto-created each time
    # v is still usable
}
```

In Rust you put `&` at the call site; in YaoXiang you put `&` at the definition site. The annotation
didn't disappear—it just moved.

---

## The Brand Mechanism: `'a` Hidden Inside the Compiler

Users never see this—but understanding it is key to understanding what YaoXiang actually did.

The compiler internally assigns each borrow token a compile-time unique number:

```
What users see         Compiler internal representation
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)
&mut Point     →  WriteToken(Point, #M)
```

When you access a field from `&Point` to get `&Float`, the latter carries a derived brand:
`#N.field_x`. When you move a `&mut` token to another variable, the compiler knows the original
variable no longer holds it—this is a basic capability of Move semantics.

**`#N` is `'a`.** Prefix relationships—`#N` is a prefix of `#N.field_x`—are outlives constraints.
The same information. Rust programmers write `'a`; YaoXiang's compiler writes `#N`.

The only difference is inference success rate. Rust's elision rules and NLL cover about 80% of
scenarios. YaoXiang's bet is: **if the language design gives the compiler cleaner input, can it
cover more?**

This bet is supported by several language constraints:

- **No variable shadowing**—`x` has only one identity in a scope, the compiler doesn't need to
  distinguish "which x did you mean"
- **Explicit `return`**—what escapes a block is written out, the compiler doesn't need to infer "is
  the last line a return value"
- **`for` creates a new binding each iteration**—variables between iterations don't interfere, the
  compiler doesn't need to track "what did the last iteration change"

These aren't "ceremony." Unlike Java's meaningless getter/setter rituals—each one turns something
the compiler would need to infer into something already written in the program. The compiler doesn't
need to guess "which variable did you refer to," "did this thing escape," "how does the loop
variable change across iterations"—the answer is in the code.

---

## What Brands Can Do—Same as Rust

Because tokens are ordinary types, they follow all ordinary type rules. No special bans like
"references can't be returned," "references can't be stored in structs," "closures can't capture
references." **But Rust is the same—all of this is achievable in Rust.** The difference isn't
capability, it's who writes the annotation.

**Returning references—Rust programmers write `'a`, YaoXiang's compiler writes `#N`:**

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()   # Token propagates to caller, compiler tracks brand derivation chain
```

Rust does the same thing, except the programmer needs to write `'a` to connect input and output. In
YaoXiang the compiler derives automatically through brand paths—`px_ref` (`#N.field_x`) derives from
`p` (`#N`). The same constraint, different notation.

**Structs holding references—no lifetime parameters:**

```yaoxiang
Window: Type = {
    target: Point,
    view: &Point,   # Token field, no different from other fields
}
```

In Rust, when structs have reference fields, `'a` needs to appear in the struct definition and all
impl blocks—the programmer explicitly annotates `Window<'a>` lifetime constraints. In YaoXiang
`view: &Point` has no `'a`, but the brand number still plays the same role internally in the
compiler—when a `Window` instance is destroyed, the tokens inside die with it. The same guarantee,
different visibility.

**Closure capture—Dup tokens copy at zero cost:**

```yaoxiang
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)   # threshold token copied into closure, zero overhead
}
```

`&Float` implements Dup (copyable), so the closure captures it like capturing a zero-sized integer.
Same rules as automatic borrowing for function calls. Zero annotation for users.

---

## The Cost: Annotations Disappear from Signatures

Rust's `'a` has a frequently cited value: it's also documentation.
`fn split_at_mut<'a>(slice: &'a mut [T], mid: usize) -> (&'a mut [T], &'a mut [T])`—the `'a` tells
the reader that the two returned slice references share the same original data.

In reality the strength of this argument is limited—most Rust beginners don't read `'a` as
documentation, they copy it as compiler-required incantation. But fairly: in complex borrowing
scenarios, Rust's `'a` at least gives a starting point for tracing data flow. In YaoXiang you'd need
to understand the brand derivation chain—and since brands are invisible to users, this depends on
tooling that isn't mature yet.

---

## Token Conflict Detection: The Same Proof Pipeline

Rust has a separate "borrow checker." YaoXiang's **design direction** is to unify borrowing
conflicts into the type-checking proof pipeline
([RFC-027 (Compile-time Predicates and Unified Static Verification)](/design/rfc/accepted/027-compile-time-evaluation-types)).

Token conflict is a Hoare proposition:

```
{ conflicting ReadToken is dead } data.push(4) { WriteToken safely acquired }
```

```yaoxiang
# &mut T is a linear type—after Move, original variable no longer holds it
bad: (p: &mut Point) -> Void = {
    p2: &mut Point = p    # WriteToken transfers from p to p2
    p.x = 10.0            # { p holds WriteToken } p.x = 10.0 { safe }
}                          # → p's WriteToken has been Moved → Disproved

# &T is Dup—copyable
good: (p: &Point) -> Void = {
    p2: &Point = p        # Copy read-only token
    print(p.x)            # OK, two read-only tokens coexist
}
```

Token conflicts share the same error reporting path as type errors and predicate verification
failures. You don't need to learn two diagnostic systems. **But the cost is:** a complex token
conflict in Rust produces a carefully worded borrow-checker error; in YaoXiang it might manifest as
"WriteToken(#7.field_x) conflicts with WriteToken(#7)"—technically accurate, but brand numbers mean
nothing to human readers. Error message explainability is unverified territory.

---

## The `ref` Keyword: Auto-Selecting Rc/Arc

Tokens cannot cross tasks (cross threads)—they are compile-time proofs, not runtime values. For
cross-scope sharing, use `ref`:

```yaoxiang
shared_data = ref Point(1.0, 2.0)   # Compiler escape analysis auto-selects Rc or Arc

spawn {
    print(shared_data.x)   # Cross-task → compiler selects Arc
}
```

- Doesn't escape into spawn block → `Rc` (non-atomic reference counting)
- Escapes into spawn block → `Arc` (atomic reference counting)

The cost: reading code locally, you can't tell if `ref` is Rc or Arc. A refactor (wrapping code in a
spawn) might silently change the reference-counting implementation—you won't get a compiler warning.
Performance changes are implicit.

---

## Current Hard Problem: RAII Is Too Coarse

Earlier we said tokens are values, lifetimes are managed by RAII. But the RAII rule for ordinary
values is: **the value lives until the scope ends.** This is precisely the problem Rust had before
NLL—the borrow persists until the end of the entire block, even if you stopped using it long ago.

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_view: &Header = data.header()    # Derive &Header from &mut Data
    header_info = parse_header(header_view) # ← last use of header_view
    # header_view doesn't need to live until function end—
    # but RAII keeps it alive until }

    data.modify(header_info)   # ❌ ReadToken is still "alive", WriteToken blocked
}
```

Rust's NLL analysis uses last use, not lexical scope. YaoXiang needs the same capability. The work
in progress integrates token liveness analysis into the proof pipeline—three layers:

1. **Fast path**—reuse existing BorrowChecker (linear scan, IR instruction level). Scenarios where
   tokens in the same basic block are released immediately after use pass directly
2. **Structural analysis**—brand tree prefix matching (determining who conflicts with whom) + DAG
   consumer queries (determining whether the token's last consumer is after the current node)
3. **SMT solving**—activated only when logical reasoning is needed, like loop conditions

The proof pipeline infrastructure (`Proved/Disproved/Unproven` three-value returns, Z3 SMT backend,
assumption stack) is partially implemented in `src/frontend/core/typecheck/proof/`. The ownership
layer (`layers/ownership.rs`) is still a skeleton—it returns Proved directly without actual
checking. Work is ongoing to fill it in.

The current version's workaround is manually nesting blocks to shorten token scope:

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_info = {
        header_view: &Header = data.header()
        parse_header(header_view)
    }   # header_view released with block end
    data.modify(header_info)   # ✅
}
```

This is real friction. Everyone coming from Rust hits it. Whether it can be eliminated—depends on
how well the pipeline integration with the ownership layer works.

---

## The Hardest Problem: The Fallback

Rust's `'a` is not just a burden—it's also a fallback. When the compiler can't infer, the programmer
annotates the lifetime relationship, and the compiler verifies. **You have a pen.**

YaoXiang's fallback theoretically should be compile-time **proof functions** (RFC-027 §4.2):
compiler auto-derivation fails → programmer writes a function whose return type is the proposition
"tokens don't conflict" → compiler verifies this function's type. But—

What does a proof function for "tokens don't conflict" look like? How does a user construct a value
of type `WriteTokenAvailable`? Do they need to understand the prefix relationship between brand
numbers `#N` and `#N.field_x`?

**If proof functions require users to understand brand numbers—we've just renamed `'a` to `#1`. We
didn't save anything.**

This problem has no answer yet. This is where the entire experiment is most likely to get stuck.

---

## Nine Iterations of RFC-009

This isn't ivory-tower design. RFC-009 went through nine major versions:

| Version | Key Changes                                                                                                                                                                               | Reason for Being Overturned                                                          |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| v1–v7   | Based on Rust's ownership model, gradually adding consumption analysis, inverse functions, field-level mutability                                                                         | Over-engineered, complexity got out of control                                       |
| **v8**  | "乞丐版借用" (乞丐 means beggar/thrift—this was the "bare-bones borrow" version)—`&T`/`&mut T` could only be parameters, couldn't be returned, stored in structs, or captured by closures | Three hardcoded bans. Severely limited expressiveness                                |
| **v9**  | Borrow token system—`&T`/`&mut T` are ordinary types following ordinary rules                                                                                                             | Eliminated special rules, but pushed brand tracking down into the compiler internals |

The v8 to v9 jump was the real breakthrough: from three bans to zero special rules. But v9
eliminated user-visible rules, not the system's inherent complexity—brand mechanism, derivation
tracking, same-source conflict detection all still exist, just moved inside the compiler. Unifying
borrow checking into the proof pipeline is a direction, but whether it actually works on real code,
and how the fallback is designed—still being validated.

---

## In Closing

We didn't eliminate `'a`. `#1` is `'a`—the same information, different location.

The experiment's bet is: language design constraints (no shadowing, explicit return, `for` new
bindings, `{}` DAG semantics) give the compiler cleaner input, and brand derivation might
automatically succeed in most scenarios where Rust's lifetime elision rules fail. If it can—users no
longer need to write `'a`, no need to distinguish annotations from elision, no need to learn the
borrow checker. If it can't—or if the fallback mechanism (proof functions) requires users to
understand brand numbers—then this is just a reinvention with a different name.

Work is ongoing. Will write again when we have results.

---

_YaoXiang is a programming language under active development. The ownership model is documented in
[RFC-009](/design/rfc/accepted/009-ownership-model), closure capture in
[RFC-023](/design/rfc/accepted/023-closure-capture-model), and the concurrency model in
[RFC-024](/design/rfc/accepted/024-concurrency-model)._
