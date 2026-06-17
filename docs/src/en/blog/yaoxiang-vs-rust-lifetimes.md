# We Hid `'a` Inside the Compiler

*— An honest note on the YaoXiang ownership model*

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

Three `'a`. One on the struct, two in the impl block. They all say the same thing: `Parser` cannot outlive the `input` it borrows. This is correct. Rust's safety is built on this mechanism.

But when we write this kind of code, there's often a thought: **can't the compiler figure this out on its own?** Rust's answer is "no"—at least not without changing the borrow model.

**This article is not "YaoXiang solved the problem." It's "YaoXiang is trying another path—hiding `'a` inside the compiler, letting it write the annotations for you. How far we've gotten, and what the hard bones are that remain unsolved."**

---

## Why Rust Needs `'a`

Rust's `&T` and `&mut T` are pointers—pointers to data. Borrowing a value means creating a reference that points to it. That reference has its own lifetime. When references propagate across function boundaries (as return values, stored in structs), the compiler cannot infer within a single function how long a reference can live—the programmer must use `'a` to provide the information that "this return value shares a lifetime with this parameter."

The Rust community hasn't stood still. Lifetime elision rules free most simple functions from annotations. NLL landed in the 2018 edition, freeing borrows from lexical scopes. But when references need to be stored in structs, returned from functions, or captured by closures—**the model itself dictates that these scenarios require the programmer to annotate relationships between references.**

---

## A Different Angle: Borrows Aren't Pointers, They're Tokens

YaoXiang's core design is documented in [RFC-009 (Ownership Model)](/design/rfc/accepted/009-ownership-model). It doesn't change the default semantics (everything is Move); it changes **what borrows fundamentally are.**

In YaoXiang, `&T` and `&mut T` **are not pointers.** They are **zero-sized compile-time tokens**—proofs of access permission at the type level. Borrowing a value doesn't create a pointer to it; it creates a "I am allowed to access it" proof:

```
&T     →  Guarantees data is immutable. Implements Dup (copyable); multiple read-only tokens coexist safely
&mut T →  Guarantees exclusive mutability. Does not implement Dup (linear); only one may exist from the same source
```

In Rust you write `&` at the call site (`distance(&p1, &p2)`). In YaoXiang, the compiler sees the function signature requires `&Point`, and automatically creates a token at the call site—the call site becomes `distance(p1, p2)`. The cost is that the definer's signature must declare `&`, otherwise the default Move semantics would consume ownership:

```yaoxiang
# & needed in the signature—compiler sees &Point, auto-creates token at call site
check_dimensions: (v: &Vec3) -> Bool = { ... }
check_bounds: (v: &Vec3) -> Bool = { ... }

v = Vec3(1.0, 2.0, 3.0)
if check_dimensions(v) && check_bounds(v) {  # each call auto-creates an &Vec3 token
    # v is still usable
}
```

In Rust you put `&` at the call site; in YaoXiang you put `&` at the definition site. The annotation hasn't disappeared—its location has changed.

---

## The Brand Mechanism: `'a` Hidden Inside the Compiler

Users never touch this—but understanding it is required to understand what YaoXiang actually does.

Internally, the compiler assigns a unique compile-time number to each borrow token:

```
What users see        What the compiler sees internally
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)
&mut Point     →  WriteToken(Point, #M)
```

When you access a field on `&Point` and get `&Float`, the latter carries a derived brand: `#N.field_x`. When you Move a `&mut` token to another variable, the compiler knows the original variable no longer holds it—this is the foundational capability of Move semantics.

**`#N` is `'a`.** The prefix relationship—`#N` is the prefix of `#N.field_x`—is the outlives constraint. The same information. Rust's programmers write `'a`; YaoXiang's compiler writes `#N`.

The only difference is inference success rate. Rust's elision rules and NLL cover roughly 80% of cases. YaoXiang's bet is: **if the language design gives the compiler cleaner inputs, can it cover more?**

This bet is supported by several language constraints:

- **No variable shadowing**—`x` has only one identity in a scope; the compiler doesn't need to distinguish "which x are you talking about"
- **Explicit `return`**—what escapes the block is written out; the compiler doesn't need to infer "is the last line the return value"
- **`for` creates a new binding per iteration**—variables across iterations don't interfere; the compiler doesn't need to track "what changed last iteration"

These are not "spec." Unlike Java's meaningless getter/setter rituals, each one of them **turns information the compiler would need to infer into information already written in the program.** The compiler doesn't have to guess "which variable are you referring to," "did this thing escape," "how do loop variables change across iterations"—the answers are in the code.

---

## What Brands Can Do—The Same as Rust

Because tokens are ordinary types, they obey all the rules of ordinary types. There are no special prohibitions like "references can't be returned," "references can't be stored in structs," "closures can't capture references." **But Rust can do all of this too.** The difference is not capability; it's who writes the annotations.

**Returning references—Rust programmers write `'a`, YaoXiang's compiler writes `#N`:**

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()   # token propagates to caller, compiler tracks brand derivation chain
```

Rust does the same thing; the programmer just needs to write `'a` connecting input and output. In YaoXiang the compiler derives automatically through brand paths—`px_ref` (`#N.field_x`) is derived from `p` (`#N`). The same constraint, recorded differently.

**Structs holding references—no lifetime parameters:**

```yaoxiang
Window: Type = {
    target: Point,
    view: &Point,   # token field, no different from other fields
}
```

In Rust, when a struct has a reference field, `'a` needs to appear in the struct definition and in every impl block—the programmer explicitly annotates `Window<'a>`'s lifetime constraint. In YaoXiang, `view: &Point` doesn't write `'a`, but the brand number still plays the same role inside the compiler—when a `Window` instance is destroyed, the token inside dies with it. The same guarantee, different visibility.

**Closure capture—zero-cost copy of Dup tokens:**

```yaoxiang
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)   # threshold token copied into closure, zero overhead
}
```

`&Float` implements Dup (copyable); capturing it in a closure is like capturing a zero-sized integer. The same rules as automatic borrowing for function calls. Zero annotations from the user.

---

## The Cost: Annotations Vanish from Signatures

Rust's `'a` has a frequently cited value: it's also documentation. `fn split_at_mut<'a>(slice: &'a mut [T], mid: usize) -> (&'a mut [T], &'a mut [T])`—`'a` tells the reader that the two returned slice references point to the same original data.

In practice, this argument has limited force—most Rust beginners don't read `'a` as documentation; they copy it as a compiler-required incantation. But to be fair: in complex borrowing scenarios, Rust's `'a` at least gives a starting point for tracing data flow. In YaoXiang, you need to understand brand derivation chains—brands are invisible to the user, which depends on tooling, and the tooling is not yet mature.

---

## Token Conflict Detection: The Same Proof Pipeline

Rust has a separate "borrow checker." YaoXiang's **design direction** is to unify borrow conflicts into the type checker's proof pipeline ([RFC-027 (Compile-Time Predicates and Unified Static Verification)](/design/rfc/accepted/027-compile-time-evaluation-types)).

Token conflict is a Hoare proposition:

```
{ conflicting ReadToken is dead } data.push(4) { WriteToken safely acquired }
```

```yaoxiang
# &mut T is a linear type—after Move, the original variable no longer holds it
bad: (p: &mut Point) -> Void = {
    p2: &mut Point = p    # WriteToken transfers from p to p2
    p.x = 10.0            # { p holds WriteToken } p.x = 10.0 { safe }
}                          # → p's WriteToken has been Moved → Disproved

# &T is Dup—copyable
good: (p: &Point) -> Void = {
    p2: &Point = p        # copy the read-only token
    print(p.x)            # OK, two read-only tokens coexist
}
```

Shares the same error reporting path as type errors and predicate verification failures. You don't need to learn two diagnostic systems. **But the cost is:** a complex token conflict in Rust produces carefully worded borrow check errors; in YaoXiang it may manifest as "WriteToken(#7.field_x) conflicts with WriteToken(#7)"—technically accurate, but brand numbers are meaningless to human readers. The interpretability of error messages is an unverified domain.

---

## The `ref` Keyword: Automatic Rc/Arc Selection

Tokens cannot cross tasks (cross-thread)—they are compile-time proofs, not runtime values. For cross-scope sharing, use `ref`:

```yaoxiang
shared_data = ref Point(1.0, 2.0)   # compiler's escape analysis auto-selects Rc or Arc

spawn {
    print(shared_data.x)   # crosses task → compiler selects Arc
}
```

- Doesn't escape into a spawn block → `Rc` (non-atomic reference counting)
- Escapes into a spawn block → `Arc` (atomic reference counting)

The cost: when reading code, you can't tell locally whether `ref` means Rc or Arc. A refactor (wrapping code in a spawn) can silently change the reference counting implementation—and you won't get a compiler warning. Performance changes are implicit.

---

## The Current Hard Bone: RAII Is Too Crude

Earlier I said tokens are values, with lifetimes managed by RAII. But the RAII rules for ordinary values are: **a value lives until the end of its scope.** This is exactly the problem Rust had before NLL—borrows persist to the end of the whole block, even when you've long since stopped using them.

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_view: &Header = data.header()    # derive &Header from &mut Data
    header_info = parse_header(header_view) # ← last use of header_view
    # header_view doesn't need to live to the end of the function—
    # but RAII keeps it alive until }

    data.modify(header_info)   # ❌ ReadToken still "alive", WriteToken is blocked
}
```

Rust's NLL analyzes the last use rather than the lexical scope. YaoXiang needs the same capability. The approach being taken is to also route token liveness analysis through the proof pipeline—three layers:

1. **Fast path**—reuse the existing BorrowChecker (linear scan, IR instruction level). Cases where a token is fully consumed within the same basic block pass through directly.
2. **Structural analysis**—brand tree prefix matching (determining who conflicts with whom) + DAG consumer queries (determining whether the token's last consumer is after the current node).
3. **SMT solving**—activated only when logical reasoning is required, e.g., loop conditions.

The proof pipeline infrastructure (`Proved/Disproved/Unproven` three-valued return, Z3 SMT backend, hypothesis stack) is partially implemented in `src/frontend/core/typecheck/proof/`. The ownership layer (`layers/ownership.rs`) is still a skeleton—it directly returns Proved, with no actual checking. Being filled in.

The current workaround is manually nesting blocks to shorten token scopes:

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_info = {
        header_view: &Header = data.header()
        parse_header(header_view)
    }   # header_view is released as the block ends
    data.modify(header_info)   # ✅
}
```

This is real friction. Everyone coming from Rust will hit it. Whether it can be eliminated depends on the effect of wiring the pipeline into the ownership layer.

---

## The Hardest Question: The Escape Hatch

Rust's `'a` isn't just a burden—it's also an escape hatch. The compiler can't infer it; the programmer annotates lifetime relationships; the compiler verifies. **You have a pen.**

YaoXiang's escape hatch should theoretically be compile-time **proof functions** (RFC-027 §4.2): when the compiler's automatic inference fails → the programmer writes a function whose return type is the proposition "tokens don't conflict" → the compiler verifies the function's type. But—

What does a "tokens don't conflict" proof function look like? How does a user construct a value of type `WriteTokenAvailable`? Do they have to understand the prefix relationship between brand numbers `#N` and `#N.field_x`?

**If the proof function requires users to understand brand numbers—then we've just renamed `'a` to `#1`. Nothing saved.**

This question has no answer yet. This is where the entire experiment is most likely to get stuck.

---

## RFC-009's Nine Iterations

This design wasn't dreamed up in an ivory tower. RFC-009 went through nine major versions:

| Version | Key Change | Why Overturned |
|---------|------------|----------------|
| v1–v7 | Based on Rust's ownership model, gradually adding consumption analysis, inverse functions, field-level mutability | Over-engineered, complexity spiraled out of control |
| **v8** | "Beggar-version borrowing"—`&T`/`&mut T` can only be parameters; cannot be returned, stored in structs, or captured by closures | Three hardcoded prohibitions. Expressiveness severely limited |
| **v9** | Borrow token system—`&T`/`&mut T` are ordinary types, following ordinary rules | Eliminated special rules, but pushed brand tracking into the compiler internals |

The leap from v8 to v9 is the real breakthrough: from three prohibitions to zero special rules. But what v9 eliminated was user-visible rules, not the system's intrinsic complexity—brand mechanisms, derivation tracking, same-source conflict detection, these all still exist, just moved into the compiler. Unifying borrow checking into the proof pipeline is a direction, but whether it can actually run on real code, and how the escape hatch should be designed—still being verified.

---

## Closing Words

We haven't eliminated `'a`. `#1` is `'a`—the same information, a different location.

The experiment's bet is: language design constraints (no shadowing, explicit return, `for` creates new binding, `{}` DAG semantics) give the compiler cleaner inputs, and brand derivation might automatically succeed in most of the cases where Rust's lifetime elision rules fail. If it can—users no longer need to write `'a`, no longer need to distinguish between annotated and elided, no longer need to learn the borrow checker. If it can't—or if the escape hatch (proof functions) requires users to understand brand numbers—then it's just a reinvention.

It's being worked on. When there are results, I'll write again.

---

*YaoXiang is a programming language under development. See the ownership model in [RFC-009](/design/rfc/accepted/009-ownership-model), closure capture in [RFC-023](/design/rfc/accepted/023-closure-capture-model), and the concurrency model in [RFC-024](/design/rfc/accepted/024-concurrency-model).*