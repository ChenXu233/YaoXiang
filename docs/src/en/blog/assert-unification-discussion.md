# An All-Night Discussion About "Unification"

From the late night of July 11, 2026 to the early morning of July 12, we spent six hours discussing
one question—what should `assert` be in YaoXiang. The result was unexpected: we thought we were
discussing "how to implement assert," but we were actually discussing "what holes exist in the type
system's foundation."

This article doesn't talk about the final conclusion (that's in the spec), but about how we step by
step discovered we were wrong, and each time how we were pulled back.

---

## Why There Are Two asserts

YaoXiang is simultaneously developing two things:

**Compile-time refinement type** `Assert(N > 0)`, written in type definitions:

```
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    _assert: Assert(N > 0),   // N must be greater than 0, otherwise compilation fails
}
```

Compile-time verification of generic parameters. If N is 5, it passes; if N is 0, it fails to
compile. Zero runtime overhead.

**Runtime assertion** `assert(x > 0)`, written in function bodies:

```
x = read_int()
assert(x > 0)   // check user input
```

Values come from outside, unknown at compile-time. Failure causes a panic.

Capital A `Assert`, lowercase `assert`. One at compile-time, one at runtime. Same concept, two
forms.

The most natural reaction—the laziest approach—is to treat them as two completely independent
things: `Assert` is a refinement type that goes through the proof pipeline, `assert` is a 20-line
native function that returns void. Each minding its own business.

But this solution always had an unspeakable discomfort. They clearly have a relationship—they're two
sides of the same semantics: "I assert this condition must hold." Making users remember "use the
capital letter here, lowercase there, this position can be checked at compile-time, that position
only at runtime" is cognitive debt. In language design, every additional "looks similar but needs to
be memorized separately" thing is taking on debt.

## We Tried to Unify, Then Hit a Wall

The ideal of unification is actually quite simple:

```
assert: (cond: Bool) -> Assert(cond)
```

`assert` returns a refinement type `Assert(cond)`. If the compile-time can compute the value of
cond, fold it into a compile-time check (erase it); if the compile-time can't compute it, leave it
for runtime checking. One function, two fates. The signature itself is the connection point.

But someone immediately pointed out a "fundamental contradiction":

> `Assert(C)` requires C to be evaluable at compile-time. In `assert(x > 0)`, x is a runtime
> value—the compile-time can't compute `Assert(x > 0)`. According to refinement type rules, this
> **must fail to compile**. But `assert(x > 0)` is a perfectly normal runtime assertion. So the
> unification scheme with the signature `Assert(cond)` doesn't work. They must be separate.

This argument sounded bulletproof. If a refinement type encounters a runtime value and reports an
error, you can never stuff "compile-time assertion" and "runtime assertion" into the same signature.
Two things, two fates, separate them.

**But it's false.** The falsehood lies in that last sentence, "must fail to compile." The result of
a refinement type is not binary—it's not just "proof established" and "error." It's three-valued:
Proved (proven), Disproved (disproven, with a counterexample), **Unproven (can't decide)**.

"Can't decide" and "disproven" are completely different states. If disproven, report an error—no way
to recover. But "can't decide"—precisely because the value is only known at runtime, so we can't
decide—can be left for runtime checking. The compile-time can't decide; that's not a compile-time
failure, it's a **natural property** of this proposition.

What is "runtime assertion" anyway? It's not "another mechanism"—it's a proposition the compile-time
can't decide, waiting until runtime to check. **The same refinement type, two fates, fate
automatically determined by the fact of whether it can be decided at compile-time.**

Once the false conflict is broken, the path to unification opens. But first we need to plug a hole
in the foundation.

## A Missing Type: Never (False / Divergent / Impossible)

The unification scheme requires `assert(false)` to reduce to a "cannot have a value" type—because
the proposition "false is true" has no proof, and where it's used as a type, the program must
diverge.

Then we discovered: YaoXiang doesn't have this type at all.

There's `Void`. But `Void` can return a default value—after calling a function that returns `Void`,
code continues. `Void` has an inhabitant. In the Curry-Howard correspondence, a type with one value
is "true proposition" (has one proof), not "false proposition" (zero proofs). **`Void` has always
been "true ⊤", we just never called it that.**

What we're missing is "false ⊥": a type that can never be inhabited—no value can fill the right side
of `x: Never = ...`. The return type of `assert(false)`, `panic`, infinite loops should all be it.
Without it, `assert(false)` would return a legitimate default void value, then continue executing
the code that follows—assertion failure becomes normal flow. The entire refinement type system
collapses logically at that moment.

`Never` was ultimately defined as an **axiom**: it's not derived from anything, it's the primordial
concept of the type system. Three properties are built into the kernel—zero constructors, Never is a
subtype of all types ("from false, anything follows"), and when used as a function return type, it
marks that function as divergent and never returning. You don't need to understand these three
rules; just know the conclusion: the language must have a built-in type called `Never`, it has no
values, it tells the system that the code after `assert(false)` can never be reached.

## We Created a Bunch of False Categories for Ourselves, Then Got Tangled Up in Them

After adding `Never`, it was time to answer "what to do when you can't decide." This step, we wasted
a full two rounds of discussion, because we made a textbook-level mistake.

**Step one: split Unproven.**

"Can't decide" seemed to have several flavors. Dependent on runtime variables—`x` is external input,
of course unknown at compile-time. Exceeded search budget—theoretically decidable, but too
expensive, gave up. Missing contextual premises—not the proposition's problem, but the caller didn't
provide enough static information. Gödel-undecidable—no matter how much time and budget,
theoretically impossible.

So we split it into four categories, gave them a bunch of names, then argued how each should be
handled. Runtime-dependent—downgrade to runtime check? Over-budget—add budget or ask user for proof?
Missing premises—add them or report error? Undecidable—user has to write a proof?

**After splitting, we found the categorization was wrong.**

The axis for categorization was chosen wrong. We were categorizing by "what the compiler should do
after getting 'can't decide.'" But "what to do" is the result of reasoning, not the basis for
categorization. The right question is: **Can the compile-time get a truth value for this
proposition's predicate?**

If it can—the parameter is in a generic position, the value is determined at compile-time, like N in
`Bounded: (T: Type, N: Int) -> Type`—go into the proof pipeline. The pipeline gives three answers:
Proved (true, erase), Disproved (false, compile error, no recovery), Unknown (truly can't decide,
but this position has compile-time ability to prove, so ask for your proof).

If it can't—the parameter is a function parameter, value passed by caller at runtime, like n in
`process: (n: Int) -> ...`—shouldn't go into the proof pipeline at all. Go directly to runtime
checking. Unproven doesn't exist on this path, because "proving" isn't even a question—you can't
write a proof for propositions like "user might have entered a negative number" because it's not
universally true.

**Just two paths. Four categories become two; the extra categorization we split out was us going in
circles.**

When this conclusion first came out, we didn't quite believe it. To verify, I gave an
example—`process: (N: Int) -> ... { assert(N > 0) }`, saying here N is compile-time known but the
prover can't decide, a new category of "can't decide + compile-time known," it should be forced to
write a proof.

**Was immediately refuted.** `process` is a regular function, `N` is a regular function
parameter—its value comes from the caller, the compile-time doesn't know it at all. I took the
`N: Int` in the function signature as "compile-time known value of N"—but in YaoXiang, "compile-time
known" precisely corresponds to **generic parameter position**, not regular function parameter
position. Same syntax `N: Int`, in a type constructor (`-> Type`) is a compile-time constant, in a
regular function is a runtime value. Two Ns, two universes.

After correction, the true face of "can't decide + compile-time known" was exposed: it's not "a
normal person wrote a proposition that the prover happens to not decide," it's **someone misused
lowercase `assert` in a generic context**—that position knows the parameter's value at compile-time,
shouldn't go through runtime checking. This isn't a new category, this is the original rule of
refinement types: in a domain requiring proof, can't decide means you should provide a proof.

**The runtime escape hatch doesn't need to be designed—it naturally exists.** It's that path: "value
unknown at compile-time, checked at runtime." Sound (really checks at runtime), convenient (don't
write prove), explicit overhead (value comes from outside anyway). It's not "we don't have a prover
so we need runtime"—it's that **this category of propositions has no truth value at compile-time at
all**—no matter how strong the prover, the user might really input a value that doesn't satisfy the
condition, the proposition itself is false, you can't write a proof for a true proposition. Runtime
is the only sound choice, theoretically necessary, not a compromise.

## Every Time We Went Wrong, the Root Cause Was the Same Thing

Looking back, we overturned nine wrong conclusions:

1. `assert` returns void `()`—that way the result of runtime check can't be passed back to
   compile-time, subsequent code doesn't know the condition holds
2. `assert` can "resolve" Unproven—no, it doesn't participate in proving, what it does is actually
   check at runtime
3. Runtime propositions and missing-context propositions are different subclasses of Unproven—no,
   they get split at the pipeline entrance, Unproven never even gets generated
4. Need to change RFC-027 to "specialized erasure"—no, what's erased is the zero-sized proof token,
   what's kept at runtime is that Bool check
5. Immutable variables go directly to compile-time—no, immutable doesn't mean compile-time known
   value (`x = read_int()` is immutable, but value is unknown at compile-time)
6. Function parameter `N: Int` is compile-time known—only generic parameters are, regular parameters
   have values only at runtime
7. With prove, no need for runtime—runtime input has no universally-true proposition to prove,
   runtime is theoretically necessary
8. "Can't decide" should default to runtime check—can't decide + compile-time known should require
   prove; what downgrades is the "compile-time doesn't know value" category
9. Type-level match is induction—match is just case analysis, no inductive hypothesis; real
   induction needs recursion + termination check

**Of the nine errors, five have the same root: confusing two concepts that look alike but are
actually orthogonal.** Immutable vs. compile-time known. Proof token vs. computation that produces
the token. Generic parameter vs. function parameter. Case analysis vs. induction. Can't decide vs.
disproven. Every pair we stepped on—not from carelessness, but because the similarity of the
concepts themselves makes it easy to slide in.

## What Kind of Question Is Worth Opening an Issue to Crowdsource Ideas

During the discussion, we opened issue #156 titled "The Unification Problem." One wrong example and
one non-existent category propped up a question that "needs community help." The core contradiction
was a false conflict, the category "can't decide + compile-time known" doesn't exist at all—it was
disguised by a wrong example. The answer actually surfaced on its own in the second half of the
discussion—**the "unification" solution isn't a compromise, it's a collapse. Every place where
"there might be a third case" eventually fell into existing paths.**

This isn't to say we can't open issues for help. It's to say confirm first: have you really seen the
real form of this problem? Does the example you gave really belong to the category you assigned it
to? Is your contradiction a real contradiction, or is it "you thought some premise held, but it
doesn't"?

Many times, "needing the community's opinion" doesn't mean the answer is far away—it means **we got
ourselves tangled up in categories we ourselves created**.

## What We Really Learned

This discussion lasted six hours; we overturned ourselves nine times. The final product isn't a new
assert API, but a foundation nailed down: `Never` must exist (no one realized it was missing
before), refinement type results are three-valued not two-valued ("can't decide" ≠ "disproven"),
compile-time known ≠ immutable, runtime checking isn't a weak replacement for prove but the only
sound choice, `assert(x > 0)` isn't "another thing" but `Assert`'s other fate in the value universe.
`assert` and `Assert` are two sides of the same thing—the difference between them isn't what they
are, but whether the compiler can compute at that moment.

**The best design is collapse, not addition.** We didn't add a "runtime assert mechanism"—we
discovered it was already one face of `Assert`. We didn't add "a handling scheme for can't
decide"—we discovered it already has a place in the pipeline. Every time you think you need to "add
a special case," ask yourself first: is this really new, or is it some existing concept you didn't
recognize?

**Categorize by essence, not by what you want to do.** At first we split four Unproven subclasses by
"what the compiler should do"—after splitting we found the method was wrong, because "what to do" is
the result of reasoning, not the basis for categorization. The right method is to split by "does the
proposition have a truth value at compile-time"—yes or no. Two categories, clean, no overlap.
