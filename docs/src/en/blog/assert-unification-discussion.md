# An All-Night Discussion About "Unification"

From late night on July 11 to the early morning of July 12, 2026, we spent six hours discussing one question—what should YaoXiang's `assert` actually be. The result was unexpected: we thought we were discussing "how to implement assert," but we were actually discussing "what holes exist in the type system's foundation."

This article doesn't talk about the final conclusion (the conclusion is in the spec), but about how we gradually discovered we were wrong, and how we were pulled back each time.

---

## Why Two asserts Exist

YaoXiang is developing two things simultaneously:

**Compile-time refinement type** `Assert(N > 0)`, written in type definitions:
```
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    _assert: Assert(N > 0),   // N must be greater than 0, otherwise compilation fails
}
```
Validates generics parameters at compile time. If N is 5, it passes; if N is 0, it's a compile error. Zero runtime overhead.

**Runtime assertion** `assert(x > 0)`, written in function bodies:
```
x = read_int()
assert(x > 0)   // check user input
```
The value comes from outside, unknown at compile time. On failure, panic.

Capital-A Assert, lowercase assert. One is compile-time, the other is runtime. Same concept, two forms.

The most natural reaction—the laziest approach—is to treat them as two completely independent things: Assert is a refinement type going through the proof pipeline, assert is a 20-line native function returning void. The waters don't mix.

But this solution always had an indescribable discomfort. They clearly have a relationship, two sides of the same semantics—"I assert this condition must hold." Making users remember "use capital here, lowercase there, compile-time check possible here, runtime only there" is cognitive debt. In language design, every "looks similar but needs to be memorized separately" element is debt incurred.

## We Tried to Unify, Then Hit a Wall

The ideal of unification is actually quite clean:

```
assert: (cond: Bool) -> Assert(cond)
```

`assert` returns a refinement type `Assert(cond)`. If the compiler can compute cond at compile time, it folds down to a compile-time check (erased); if it can't, it leaves it for runtime. One function, two fates. The signature itself is the connection point.

But someone immediately pointed out a "fundamental contradiction":

> `Assert(C)` requires C to be evaluable at compile time. In `assert(x > 0)`, x is a runtime value—the compiler cannot compute `Assert(x > 0)`. By the rules of refinement types, this **must produce a compile error**. But `assert(x > 0)` is a perfectly normal runtime assertion. So a unified signature of `Assert(cond)` doesn't work. They must be separate.

This argument sounds bulletproof. If a refinement type throws an error when it encounters a runtime value, you can never fit "compile-time assertion" and "runtime assertion" into the same signature. Two things, two fates—separate them.

**But it's false.** The falsity lies in that last sentence "must produce a compile error." The result of a refinement type isn't binary—it's not just "proof holds" and "error." It's trinary: Proved (proven), Disproved (refuted, with counterexample), **Unproven (can't decide)**.

"Can't decide" and "refuted" are completely different states. Refuted should error out, with no way to recover. But "can't decide"—because the value is only known at runtime, so it can't be decided—is precisely what can be left for runtime checking. The compiler not being able to decide isn't a compile-time failure; it's the proposition's **inherent property**.

What is "runtime assertion" anyway? It's not "another mechanism"—it's a proposition that the compiler can't decide, waiting until runtime to be checked. **The same refinement type, two fates, the fate automatically determined by the fact of whether it can be decided at compile time.**

Once the false conflict was broken, the path to unification opened. But we first had to fill a hole in the foundation.

## A Missing Type: Never (False / Divergent / Impossible)

The unified solution requires `assert(false)` to reduce to an "impossible to have a value" type—because "false is true" has no proof, and any place using it as a type must diverge.

Then we discovered: YaoXiang has no such type at all.

It has `Void`. But `Void` can return a default value—call a function returning `Void`, and the code continues afterward. `Void` has an inhabitant. In the Curry-Howard correspondence, a type with one value is the "true proposition" (one proof), not the "false proposition" (zero proofs). **`Void` has always been "true ⊤", just nobody called it that.**

What we're missing is "false ⊥": a type that can never be inhabited—no value can fit on the right side of `x: Never = ...`. `assert(false)`, `panic`, the return type of an infinite loop—all should be it. Without it, `assert(false)` would return a legal default void value, and the code would keep running—an assertion failure becoming normal flow. The entire refinement type system collapses logically at that point.

`Never` was ultimately defined as an **axiom**: it's not derived from anything; it's the type system's primordial concept. Three properties are built into the kernel—zero constructors, Never is a subtype of all types ("from false, anything follows"), and as a function return type, it marks the function as divergent and never returning. You don't need to understand these three properties—just know the conclusion: the language must have a built-in type called `Never`, it has no values, and it tells the system that the code after `assert(false)` can never be reached.

## We Manufactured a Pile of False Classifications, Then Walked Into Them Ourselves

After filling in Never, the next step was answering "what to do when it can't be decided." This step wasted us two full rounds of discussion, because we made a textbook-level mistake.

**Step one: splitting Unproven.**

"Undecided" seemed to have several flavors. Dependent on runtime variables—`x` is external input, of course the compiler doesn't know. Over the search budget—theoretically decidable, but computation was too expensive, so it was abandoned. Missing contextual premises—not a problem with the proposition itself, but the caller didn't provide enough static information. Gödel-undecidable—no amount of time or budget could decide it, theoretically impossible.

So we split it into four categories, gave them a bunch of names, and argued about how to handle each. Runtime-dependent—downgrade to runtime check? Over-budget—add budget or ask the user for proof? Missing premises—add them or error out? Undecidable—user has to write a proof?

**After splitting, we found the classification was wrong.**

The axis of classification was wrong. We were classifying by "what should the compiler do after it can't decide." But "what to do" is a result of reasoning, not a basis for classification. The right question is: **Can the compiler obtain a truth value for this proposition's predicate at compile time?**

If it can—the parameter is in the generics position, the value is determined at compile time, like N in `StaticArray: (T: Type, N: Int) -> Type`—it enters the proof pipeline. The pipeline gives three answers: Proved (true, erase), Disproved (false, compile error, no recovery), Unknown (truly undecidable, but at this position the compiler has the ability to prove, so ask for your proof).

If it can't—the parameter is a function argument, the value is passed in by the caller at runtime, like n in `process: (n: Int) -> ...`—it shouldn't enter the proof pipeline at all. Go directly to runtime check. Unproven doesn't exist on this path, because there's no "proof" question—you can't write a proof for "the user might have input a negative number," because the proposition isn't universally true.

**Just two paths. Four became two, and the extra classifications we split out were us tangling ourselves.**

When this conclusion first came out, we didn't quite believe it. To verify, I gave an example—`process: (N: Int) -> ... { assert(N > 0) }`—saying here N is known at compile time but the prover can't decide it, a new "undecidable + compile-time known" category, and a proof should be forced.

**It was refuted on the spot.** `process` is an ordinary function, `N` is an ordinary function parameter—its value comes from the caller, the compiler doesn't know it at all. I was treating the `N: Int` in the function signature as "N's value is known at compile time"—but in YaoXiang, "known at compile time" precisely corresponds to the **generics parameter position**, not the ordinary function parameter position. Same syntax `N: Int`, in a type constructor (`-> Type`) it's a compile-time constant, in an ordinary function it's a runtime value. Two N's, two universes.

After correction, the true face of "undecidable + compile-time known" was exposed: it's not "a normal person wrote a proposition that happens to be undecidable by the prover"—it's **someone misused lowercase `assert` in a generics context**—at that position, the compiler knows the parameter's value, and runtime checking shouldn't be used. This isn't a new category; this is the refinement type's own rule: when you're in a domain that requires proof, the inability to decide should mean providing a proof.

**The runtime escape hatch doesn't need to be designed—it exists naturally.** It's the path "the value is unknown at compile time, so we check at runtime." Sound (runtime really does check), convenient (no prove to write), explicit cost (the value comes from outside anyway). It's not "we don't have a prover, so we need runtime"—it's that **this class of propositions has no truth value at compile time to speak of**—no matter how strong the prover is, the user might really input a value that doesn't satisfy the condition, the proposition is false in itself, you can't write a proof of a true proposition. Runtime is the only sound choice, theoretically necessary, not a compromise.

## Every Wrong Turn, the Root Cause Was the Same Thing

Looking back, we refuted nine wrong conclusions:

1. `assert` returns the empty value `()`—then the runtime check result can't be transmitted back to compile time, and subsequent code doesn't know the condition is established
2. `assert` can "dissolve" Unproven—no, it doesn't participate in proof, it does the actual runtime check
3. Runtime propositions and missing-context propositions are different subclasses of Unproven—no, they get split at the pipeline entrance, they don't even produce Unproven
4. RFC-027 needs to be changed to "specialized erasure"—no, what's erased is the zero-sized proof token, what runtime keeps is that Bool check
5. Immutable variables go directly to compile time—no, immutable doesn't equal compile-time known value (`x = read_int()` is immutable, but the value is unknown at compile time)
6. Function parameter `N: Int` is known at compile time—only generics parameters are, ordinary parameter values only exist at runtime
7. With prove, you don't need runtime—runtime input has no universally true proposition to prove, runtime is theoretically necessary
8. "Undecidable" should default to runtime check—undecidable + compile-time known should demand prove, what downgrades is the "compiler doesn't know the value" class
9. Type-level match is induction—match is just case analysis, no inductive hypothesis; real induction needs recursion + termination check

**Of the nine errors, five had the same root: confusing two concepts that look similar but are actually orthogonal.** Immutable vs. compile-time known. Proof token vs. the computation that produces the token. Generics parameter vs. function parameter. Case analysis vs. induction. Undecidable vs. disproved. We stepped on every one of these pairs—not from carelessness, but because the concepts' similarity makes it extremely easy to slide in.

## What Kind of Question Is Worth Opening an Issue For Community Discussion

During the discussion, we opened issue #156, titled "The Unification Dilemma." One wrong example and one non-existent classification propped up a "need community help" question. The core contradiction was a false conflict; the "undecidable + compile-time known" category doesn't exist at all—it was masquerading as one through a wrong example. The answer actually surfaced by itself in the second half of the discussion—**the "unified" solution isn't a compromise, it's a collapse. Every place where we thought "maybe there's a third case" eventually fell into an existing path.**

This isn't saying you can't open issues for help. It's saying you need to confirm first: Have you really seen the true form of this problem? Does the example you gave really belong to the class you assigned it to? Is your contradiction a real contradiction, or is it "you thought some premise was true, but it isn't"?

Often, "needing the community's opinion" isn't because the answer is too far—it's that **you've tangled yourself in classifications you made up yourself**.

## What We Really Learned

This discussion lasted six hours, and we refuted ourselves nine times. The final product isn't a new assert API, but a foundation that's been nailed down: `Never` must exist (no one realized it was missing before), the result of a refinement type is trinary not binary ("undecidable" ≠ "disproved"), compile-time known ≠ immutable, runtime checking isn't a weak substitute for prove but the only sound choice, `assert(x > 0)` isn't "another thing"—it's `Assert`'s other fate in the value universe. assert and Assert are two sides of the same thing—the difference between them isn't what they are, it's whether the compiler can compute at that moment.

**The best design is collapse, not addition.** We didn't add a "runtime assert mechanism"—we discovered it was always one side of `Assert`. We didn't add "a way to handle undecidable"—we discovered it already has a place in the pipeline. Every time you think you need to "add a special case," ask yourself first: is this really new, or is it some existing concept you didn't recognize?

**Classify by essence, not by what you want to do.** We initially split four Unproven subclasses by "what the compiler should do"—after splitting, we found the classification was wrong, because "what to do" is a result of reasoning, not a basis for classification. The right method is to classify by "whether the proposition has a truth value at compile time"—yes or no, that's it. Two categories, clean, no overlap.