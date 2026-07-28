# An All-Night Conversation About "Unification"

July 11-12, 2026, late night to early morning. Six hours discussing one question—what should YaoXiang's `assert` be? The result was surprising: we thought we were discussing "how to implement assert," but we were actually discussing "what holes exist in the type system's foundation."

This article doesn't cover the final conclusion (that's in the spec). It covers how we step by step discovered we were wrong, and what pulled us back each time.

---

## Why Two asserts Exist

YaoXiang is developing two things simultaneously:

**Compile-time refinement type** `Assert(N > 0)`, written in type definitions:

```
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    _assert: Assert(N > 0),   // N must be greater than 0, or compile fails
}
```

Compile-time validation of generics parameters. N is 5, it passes. N is 0, compile error. Zero runtime overhead.

**Runtime assertion** `assert(x > 0)`, written in function bodies:

```
x = read_int()
assert(x > 0)   // Check user input
```

The value comes from outside, compile-time doesn't know. Failure triggers panic.

Capital Assert, lowercase assert. One compile-time, one runtime. The same concept, two forms.

The most natural reaction—the laziest approach—is to treat them as two completely independent things: Assert is a refinement type that goes through the proof pipeline, assert is a 20-line native function returning void. Two separate things, no overlap.

But this solution always felt unpleasantly uncomfortable. They're clearly related—they're two faces of the same semantic concept—"I assert this condition must hold"—and making users remember "use uppercase here, lowercase there, this one gets compile-time checking, that one only at runtime" is cognitive debt. In language design, every "looks similar but must be memorized separately" thing is debt.

## We Tried to Unify, Then Hit a Wall

The ideal of unification is actually quite simple:

```
assert: (cond: Bool) -> Assert(cond)
```

`assert` returns a refinement type `Assert(cond)`. If compile-time can evaluate cond, it folds into a compile-time check (erased); if it can't, it stays for runtime checking. One function, two fates. The signature itself is the connection point.

But someone immediately pointed out a "fundamental contradiction":

> `Assert(C)` requires C to be evaluable at compile-time. In `assert(x > 0)`, x is a runtime value—`Assert(x > 0)` cannot be computed at compile-time. According to refinement type rules, this **must be a compile error**. But `assert(x > 0)` is a perfectly normal runtime assertion. So the unified scheme with signature `Assert(cond)` doesn't work. They must be separate.

This argument sounds airtight. If refinement types error on runtime values, you can never put "compile-time assertion" and "runtime assertion" into the same signature. Two things, two fates, separate.

**But it's false.**
The falsity is in "must be a compile error." The result of a refinement type isn't binary—it's not just "proved" and "error." It's three-valued: Proved (proven), Disproved (falsified with counterexample), **Unproven (cannot be determined)**.

"Unproven" and "Disproved" are completely different states. Disproved should error, never to recover. But Unproven—exactly because the value is only known at runtime so it can't be determined—is precisely what can be deferred to runtime checking. Can't determine at compile-time, not a compile-time failure, it's a **natural property of that proposition**.

"RUNTIME ASSERTION" isn't "another mechanism" at all—it's a proposition that compile-time can't determine, waiting until runtime to be checked. **The same refinement type, two fates, the fate automatically determined by whether it can be decided at compile-time.**

Once this false conflict is broken, the path to unification opens. But we must first patch a hole in the foundation.

## Missing a Type: Never (False / Divergence / Impossible)

The unified scheme requires `assert(false)` to reduce to a type that "can never have a value"—because the proposition "false is true" has no proof; places using it as a type must diverge.

Then we discovered: YaoXiang doesn't have this type at all.

There's `Void`. But `Void` can return a default value—calling a function returning `Void`, after the call code continues. `Void` has an inhabitant. In Curry-Howard correspondence, a type with a value corresponds to "true proposition" (one proof exists), not "false proposition" (zero proofs). **`Void` has always been "truth ⊤"; it's just no one called it that.**

What we lacked is "false ⊥": a type that can never be inhabited—no value can fill the right side of `x: Never = ...`. `assert(false)`, `panic`, the return type of infinite loops should all be this. Without it, `assert(false)` would return a legitimate default void value, then continue executing—assertion failure becomes normal flow. The entire refinement type system collapses logically at that moment.

`Never` was ultimately established as an **axiom**: it's not derived from anything, it's a primitive concept in the type system. Three properties built into the kernel—zero constructors, Never is a subtype of all types ("from false, anything follows"), as a function return type it marks that function as diverging and never returning. You don't need to understand these three properties; just know the conclusion: the language must have a built-in type called `Never` with no values, making it clear that code after `assert(false)` can never execute.

## We Created a Bunch of False Categories, Then Got Lost in Them

After filling in Never, we needed to answer "what to do when unproven." This step wasted整整 two rounds of discussion because we made a textbook-level mistake.

**Step One: Split Unproven.**

"Unproven" looked like it had several cases. Runtime-dependent—x is external input, compile-time obviously doesn't know. Over-budget— theoretically can be determined, but too expensive so gave up. Missing-context prerequisites—not the proposition's problem, caller didn't provide enough static information. Gödel-undecidable—more time and budget wouldn't help, theoretically impossible.

So we split it into four types, came up with a bunch of names, then argued about how each should be handled. Runtime-dependent—downgrade to runtime check? Over-budget—increase budget or ask user for proof? Missing-prerequisites—add prerequisites or error? Undecidable—user must write proof?

**After splitting, we discovered the classification was wrong.**

The axis of classification was wrong. We split by "what should the compiler do after encountering unproven." But "what to do" is the result of reasoning, not the basis for classification. The right question is: **can the predicate of this proposition get a Boolean value at compile-time?**

If it can—parameter is in generics position, value is compile-time definite, like N in `StaticArray: (T: Type, N: Int) -> Type`—it goes into the proof pipeline. Pipeline gives three answers: Proved (true, erased), Disproved (false, compile error never recovers), Unknown (truly undecidable, but compile-time has the capacity to prove in this position, so asking for proof).

If it can't—parameter is a function parameter, value passed by caller at runtime, like n in `process: (n: Int) -> ...`—it shouldn't enter the proof pipeline at all. Goes straight to runtime checking. On this path, Unproven doesn't exist, because there's no "proof" problem—you can't write a proof for "user might have input a negative number" because it's not universally true.

**Just two paths. Four becomes two; the excess categories were just us chasing our own tails.**

When this conclusion first came out, we didn't fully believe it. To verify, I gave an example—`process: (N: Int) -> ... { assert(N > 0) }`—arguing that N is compile-time known but prover can't decide it, forming a new category of "unproven + compile-time known" that should force proof writing.

**It got shot down immediately.**
`process` is an ordinary function, `N` is an ordinary function parameter—its value comes from the caller, compile-time doesn't know at all. I took the `N: Int` in the function signature as "N's value is compile-time known"—but in YaoXiang, "compile-time known" precisely corresponds to **generic parameter position**, not ordinary function parameter position. The same syntax `N: Int`, in type constructor (`-> Type`) is compile-time constant, in ordinary function is runtime value. Two Ns, two universes.

After correction, the true face of "unproven + compile-time known" was exposed: not "normal person wrote a proposition prover can't handle," but **someone mistakenly used lowercase `assert` in generics context**—that position compile-time knows the parameter value, shouldn't go to runtime checking. This isn't a new category; it's just refinement type's original rule: being in a domain requiring proof, can't prove means provide proof.

**The runtime escape hatch doesn't need to be designed—it naturally exists.**
It's just the path of "value not known at compile-time, check at runtime." Sound (runtime actually checks), convenient (no proof writing), explicit overhead (value comes from outside anyway). Not "we don't have prover so we need runtime," but **this class of propositions has no truth value at compile-time by nature**—no matter how strong the prover, user might actually input a value not satisfying the condition, the proposition is false, can't write a proof of a true proposition. Runtime is the only sound choice, theoretically necessary, not a compromise.

## Every Wrong Turn, the Root Cause Was the Same

Looking back, we overturned nine wrong conclusions:

1. `assert` returns void `()`—then runtime check result can't propagate back to compile-time, subsequent code doesn't know condition has been established
2. `assert` can "resolve" Unproven—no, it doesn't participate in proof, what it does is actual runtime checking
3. Runtime propositions and missing-context propositions are different subclasses of Unproven—no, they're split at the pipeline entrance, they never generate Unproven
4. Need to change RFC-027 to "specialization erasure"—no, what's erased is the zero-sized proof token, what's retained at runtime is that Bool check
5. Immutable variables directly go to compile-time—no, immutable doesn't mean compile-time known (`x = read_int()` is immutable, but value unknown at compile-time)
6. Function parameter `N: Int` is compile-time known—only generic parameters are, ordinary parameters' values come at runtime
7. If we have prove we don't need runtime—runtime inputs have no universally-true propositions to prove, runtime is theoretically necessary
8. "Unproven" should default to runtime checking—unproven + compile-time known should require proof, downgrade is for "value not known at compile-time"
9. Type-level match is just induction—no, match is just case analysis without induction hypotheses; true induction needs recursion + termination checking

**Five of nine mistakes, same root: confusing two concepts that look similar but are actually orthogonal.**
Immutable vs. compile-time known. Proof token vs. the computation that produces it. Generic parameter vs. function parameter. Case analysis vs. induction. Unproven vs. disproven. We've stepped on every pair—not carelessness, but conceptual similarity makes it extremely easy to slide into confusion.

## What Kinds of Problems Deserve an Issue for Brainstorming

During discussion, we opened issue#156 titled "The Unification Problem." A wrong example and a non-existent category supported a "need community help" problem. The core conflict was a false one, "unproven + compile-time known" doesn't exist as a category at all—disguised by a wrong example. The answer had actually emerged halfway through the discussion—**the "unification" scheme isn't a compromise, it's a collapse. Every place we thought "maybe there's a third case," ultimately fell into existing paths.**

Not saying issues shouldn't be opened for help. But first confirm: have you actually encountered the real form of this problem? Does your example actually belong in the category you placed it in? Is your conflict real, or is it "you assumed some premise held, but it doesn't"?

Often, "need community input" doesn't mean the answer is too far away—it's **you've tangled yourself in categories you invented**.

## What We Actually Learned

The discussion lasted six hours, overturning ourselves nine times. The final product isn't a new assert API, but a pinned foundation: `Never` must exist (no one realized it was missing), refinement types have three values not two ("unproven" ≠ "disproved"), compile-time known ≠ immutable, runtime checking isn't a weak substitute for prove but the only sound choice, and `assert(x > 0)` isn't "another thing" but `Assert` in the value universe's other fate. assert and Assert are two sides of the same thing—the distinction isn't what they are, but whether the compiler can compute at that moment.

**The best design collapses, it doesn't add.** We didn't add a "runtime assert mechanism"—we discovered it was already a face of `Assert`. We didn't add "a solution for unproven"—we discovered it already had a place in the pipeline. Every time you feel the need to "add a special case," first ask yourself: is this truly new, or have you failed to recognize an existing concept?

**Classify by essence, not by what you want to do.** We initially split Unproven into four subclasses by "what should the compiler do"—but when we finished splitting, we discovered the approach was wrong, because "what to do" is the result of reasoning, not the basis for classification. The correct method classifies by "does the proposition have a truth value at compile-time"—if it does, it does; if not, it doesn't. Two categories, clean, no overlap.
