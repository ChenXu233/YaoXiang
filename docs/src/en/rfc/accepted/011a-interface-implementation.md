---
title: 'RFC-011a: Interface Implementation and Dynamic Dispatch'
status: 'accepted'
author: 'Chenxu'
created: '2026-06-14'
updated: '2026-08-19'
group: 'rfc-011'
---

# RFC-011a: Interface Implementation and Dynamic Dispatch

> **Parent RFC**: [RFC-011: Generic System Design](011-generic-type-system.md)
>
> **This RFC supplements and replaces the interface constraint portion of RFC-011 §2.1-2.4.**

## Summary

RFC-011 defined the generics system but did not detail the interface implementation mechanism. This
document supplements:

1. **Interface declaration**: interfaces are parameterized types — `(Self: Type) -> Type`,
   instantiated with concrete types
2. **Method implementation**: both internal and external declarations are supported
3. **Overload rules**: different signatures allow overloading, same signature errors (override
   forbidden)
4. **Default values**: write `= value` directly after a field
5. **Dynamic dispatch**: compile-time type collection + interface matching, no vtable

**Core design**:

```yaoxiang
# Interface definition (parameterized type, Self is an explicit type parameter)
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition (internal declaration)
Dog: Type = {
    x: Int = 10,
    Animal(Dog),  # Interface instantiation, Self ↦ Dog
    speak: (self: &Dog) -> String = "Woof",
}

# External declaration (overload)
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# Heterogeneous container (dynamic dispatch)
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**Receiver spelling convention** (cooperating with RFC-009 ownership semantics):

- The method receiver follows the signature semantics: `&Self` = borrow (the default convention for
  interfaces — method calls do not consume the receiver), `&mut Self` = mutable borrow, by-value
  `Self` = consume the receiver (Move, RFC-009).
- In the impl-side signature, `Self` is an alias for the impl type: the interface
  `speak: (self: &Self)` matches both impl `(self: &Dog)` and `(self: &Self)` (after Self ↦
  impl-type substitution they are exactly identical, §3).
- The by-value receiver spelling in historical examples (`(self: Self)`) meant borrow, and this
  document has uniformly migrated it to explicit `&Self`; by-value spelling now reserves the
  "consume" semantics, no longer mixed.

**Eliminated complexity**:

- ❌ No `impl` keyword
- ❌ No `Self` magic keyword (`Self` is an explicit type parameter, no different from `T`)
- ❌ No `dyn Trait + 'a` annotations
- ❌ No vtable (compile-time type collection + enum wrapping)
- ❌ No override (uniform overload rules)

---

## Motivation

### RFC-011's Deficiencies

RFC-011 defined the generics system but did not detail:

| Problem                        | Description                                         |
| ------------------------------ | --------------------------------------------------- |
| Interface declaration syntax   | How to declare that a type implements an interface? |
| Method implementation location | Internal declaration or external declaration?       |
| Overload rules                 | How to handle same-named methods?                   |
| Default value syntax           | How do fields set default values?                   |
| Dynamic dispatch               | How to implement heterogeneous containers?          |

### Design Goals

1. **Concise**: no `impl` keyword needed
2. **Flexible**: method implementation supports both internal and external declarations
3. **Unified**: overload rules are consistent
4. **Convenient**: default value syntax is concise
5. **Zero overhead**: no vtable, compile-time type collection

### Comparison with Rust

| Feature                 | Rust                                   | YaoXiang                              |
| ----------------------- | -------------------------------------- | ------------------------------------- |
| Interface declaration   | `impl Animal for Dog { ... }`          | `Dog: Type = { Animal(Dog), ... }`    |
| Method implementation   | in `impl` block                        | internal or external                  |
| Overload                | not supported                          | supported (different signatures)      |
| Default value           | requires `#[default]`                  | write `= value` directly              |
| Heterogeneous container | `Vec<Box<dyn Animal + 'a>>`            | `List(Animal)`                        |
| Dynamic dispatch        | vtable lookup                          | compile-time type collection          |
| `Self` keyword          | magic keyword, implicit quantification | explicit type parameter, equal to `T` |

---

## Proposal

### 1. Interface Declaration

**Core rule**: an interface is a parameterized type `(Self: Type) -> Type`; `Self` is an explicit
type parameter, not a magic keyword. When implementing, call the interface and pass in a concrete
type.

```yaoxiang
# Interface definition (consistent with RFC-011 generic types)
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type declaration implements interface
Dog: Type = {
    x: Int,
    Animal(Dog),  # Instantiate interface, Self ↦ Dog
}
```

**Compiler processing**:

1. Recognize `Animal(Dog)` as an instantiation call of `(Self: Type) -> Type`
2. Perform `Self ↦ Dog` substitution: expand `Animal(Dog)` → `{ speak: (self: &Dog) -> String }`
3. Check whether `Dog` provides all required methods (signature match)
4. If passed → generate implementation proof
5. If failed → compile error

**Expansion equivalence**:

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),  # Expand as Animal's methods, retain source marker
}

# Equivalent to (preserving source info)
Dog: Type = {
    x: Int,
    speak: (self: &Dog) -> String,  # from Animal, Self replaced by Dog
}
```

**Why source markers are needed**:

- Direct expansion loses source information
- Source markers are used to generate implementation proof
- Runtime uses the proof to find the correct method

#### 1.1 `Self` Type Parameter and Type-Checking Timing

`Self` is an explicit type parameter of the interface, not a magic keyword.
`Animal: (Self: Type) -> Type` and `List: (T: Type) -> Type` are the same thing — a `(Type) -> Type`
type constructor.

**Type-checking timing**:

- **At interface definition**: `Self` in `{ speak: (self: &Self) -> String }` is an abstract type
  parameter, only syntax-checked.
- **At instantiation point**: when `Animal(Dog)` is called, perform `Self ↦ Dog`, then run a
  complete type check after expansion (signature match, method existence).

This avoids the problem of `Self` as an implicit magic keyword in RFC-011 — `Self` does not appear
in type definitions; it only appears once in the interface parameter list, fully equal to `T`.

#### 1.2 Field Name and Method Name Namespace

A type's field names and method names share the same namespace. After interface expansion, if the
interface method name conflicts with a type field name, **compile error**:

```yaoxiang
Drawable: (Self: Type) -> Type = {
    x: (self: &Self) -> Int,    // method named x
}

Point: Type = {
    x: Int,                     // field also named x
    Drawable(Point),            // ❌ compile error: Drawable requires method x, conflicts with field x
}
```

Field access `point.x` and method call `point.x()` are syntactically indistinguishable. A unified
namespace avoids ambiguity.

### 2. Method Implementation

**Core rule**: method implementation supports both internal and external declarations.

#### 2.1 Internal Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # method implementation inside
}
```

#### 2.2 External Declaration

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),
}

# Method implementation outside
Dog.speak: (self: &Dog) -> String = "Woof"
```

#### 2.3 Mixed Declaration

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # part of methods inside
}

# Part of methods outside
Dog.play: (self: &Dog) -> Void = { ... }
```

**Compiler processing**:

1. Collect all definitions (internal and external)
2. Group by signature (overload)
3. Check for override (error)
4. Check interface completeness
5. Generate implementation proof

### 3. Overload and Override

**Core rule**:

- Different signatures → overload → allowed
- Same signature → override → error

#### 3.1 Overload (allowed)

```yaoxiang
# Different parameter types, overload allowed
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"
```

#### 3.2 Override (forbidden)

```yaoxiang
# Identical signature, override forbidden
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ error: override not allowed
```

**Error message**:

```
error: duplicate definition of Dog.speak(self: &Dog) -> String
  --> file2:5:1
  |
5 | Dog.speak: (self: &Dog) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ duplicate definition
  |
  --> file1:3:1
  |
3 | Dog.speak: (self: &Dog) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ first definition
```

#### 3.3 Unified Rules

**Internal and external declarations follow the same overload/override rules**:

```yaoxiang
# Internal declaration
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

# External declaration (overload, allowed)
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# External declaration (override, forbidden)
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ error
```

### 4. Default Values

**Core rule**: write `= value` directly after a field, eliminating the need for constructors.

```yaoxiang
Dog: Type = {
    x: Int = 10,  # default value
    y: Int = 20,  # default value
    Animal(Dog),
}
```

**Compiler generates constructors**:

```yaoxiang
# All fields have default values → generate no-arg constructor
Dog.new: () -> Dog = { x: 10, y: 20 }

# Some fields have default values → generate partial-arg constructors
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# Full-arg constructor
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**External default declaration**:

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal(Dog),
}

# External default declaration
Dog.x: Int = 10
Dog.y: Int = 20
```

**Equivalent to internal declaration**.

### 5. Compiler Implementation

#### 5.1 Interface Descriptor

```rust
// Compiler internal: interface descriptor
struct InterfaceDescriptor {
    name: String,
    self_param: TypeParam,     // Self type parameter
    methods: Vec<MethodSignature>,
}
```

#### 5.2 Type Definition

```rust
// Compiler internal: type definition
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_instantiations: Vec<InterfaceInstantiation>,
}

// Interface instantiation (Self ↦ ConcreteType)
struct InterfaceInstantiation {
    interface: InterfaceId,
    self_type: TypeId,          // Concrete type Self is substituted with
    methods: HashMap<MethodId, FunctionBody>,
}
```

#### 5.3 Implementation Proof

```rust
// Compiler internal: implementation proof
struct ImplementationProof {
    type_id: TypeId,
    interface_id: InterfaceId,
    methods: Vec<MethodPointer>,
}
```

#### 5.4 Compilation Flow

```
1. Parse type definitions, collect interface instantiation declarations (Animal(Dog))
2. For each interface instantiation, perform Self ↦ ConcreteType substitution
3. Expand interface method signatures, check signature match
4. Collect all method definitions (internal and external)
5. Group by signature (overload)
6. Check override (error)
7. Check interface completeness
8. Generate implementation proof
```

### 6. Dynamic Dispatch

**Core design**: compile-time type collection + interface matching, no vtable.

#### 6.1 Heterogeneous Container

`Animal` is `(Self: Type) -> Type`. `List(Animal)` uses the uninstantiated interface type
constructor as an **existential type**: `∃S. Animal(S)` — "there exists some type S such that S
implements `Animal(S)`".

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: &Cat) -> String = "Meow",
}

# Heterogeneous container — Animal uninstantiated = existential type
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

**Ownership semantics**: putting into a heterogeneous container is Move semantics (RFC-009).
`Dog.new()` is moved into the `AnimalGroup::Dog` enum variant; the original variable is no longer
usable.

```yaoxiang
dog = Dog.new()
animals: List(Animal) = [dog]
# dog.speak()  ← ❌ compile error: dog has been moved
```

#### 6.2 Compile-time Type Collection

**Core strategy: ownership tracking, incremental construction.** Rather than scanning all types
implementing the interface at compile time — at each **ownership operation point** of
`List(Animal)`, collect incrementally:

```yaoxiang
// Construction point
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append point
animals.append(Cat.new())                  // compiler sees Cat at append → extends to { Dog, Cat }
animals.append(Bird.new())                 // extends again { Dog, Cat, Bird }
```

**Compiler processing** (incremental):

1. Encounter the first construction of `List(Animal)` → generate the initial enum (all currently
   known construction types within the compilation unit)
2. Each `append` / `push` / index assignment → check whether the value type is already in the enum;
   if not, extend the enum variants
3. Generate monomorphized `match` dispatch code for the final enum
4. Cross-compilation-unit: rely on LTO (link-time optimization) to merge enum variants. When
   `Animal` is passed across compilation unit boundaries as an existential type, each unit generates
   partial enum variants, and the link phase merges them into a complete enum.

**Auto-generated enum**:

```yaoxiang
# Compiler auto-generated (not visible to the user)
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) triggers incremental extension
}

# List(Animal) is internally equivalent to List(AnimalGroup)
```

#### 6.3 Interface Match Checking

**Key insight**: interface matching is checked at compile time, even if the type comes from a
dynamically loaded plugin.

```yaoxiang
# Plugin system
plugin = load_plugin("bird.so")

# Compiler checks: plugin.create_bird()'s return type must implement Animal
bird: Animal = plugin.create_bird()  # compile-time check, existential type

# Put into heterogeneous container — append point triggers enum extension
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 // compiler: (1) verify bird implements Animal (2) extend enum
```

**Compiler processing**:

1. Check the return type of the `append` argument
2. Verify that the type implements the target interface
3. If passed → extend the enum and allow placement
4. If failed → compile error

#### 6.4 Runtime Dispatch

**Call flow (compile-time enum match, `ImplementationProof` already erased):**

```
animals[0].speak()
  ↓
Compiler-generated match:
  match animals[0] {
    AnimalGroup.Dog(d) => d.speak(),
    AnimalGroup.Cat(c) => c.speak(),
    AnimalGroup.Bird(b) => b.speak(),
  }
```

**Brand projection** (interaction with RFC-009a): the pattern binding `AnimalGroup.Dog(d)` in the
`match` produces a `#animals[0].Dog` sub-brand in the brand tree, equivalent to field projection
(`#42.field_x`). The `ReadToken(d)` brand chain created by `d.speak()` is
`animals → animals[0] → d → ReadToken(d)`; the borrow checker verifies conflicts via brand-tree
prefix matching.

**Type of index access**: `animals[0]` returns `&AnimalGroup` (the compiler-generated enum type);
the user cannot directly obtain `&mut Animal`. Mutable access is implemented indirectly through
interface methods (e.g. `animals[0].mutate()` internally expands to
`AnimalGroup::Dog(d) => d.mutate()`).

**Comparison with vtable**:

|                       | Vtable (Rust)                   | Compile-time enum (YaoXiang)              |
| --------------------- | ------------------------------- | ----------------------------------------- |
| Lookup method         | vtable pointer → method pointer | enum `match` → direct call                |
| Runtime overhead      | one indirection                 | branch (CPU branch predictor friendly)    |
| Compile-time artifact | vtable                          | enum + `match`                            |
| User annotation       | requires `dyn Trait + 'a`       | none                                      |
| `ImplementationProof` | N/A                             | compile-time erased, no runtime existence |

**YaoXiang's advantages**:

- No brand annotation needed
- Compile-time type safety
- User-transparent (no need to write `dyn Animal`)
- `ImplementationProof` is a pure compile-time concept, zero runtime overhead

#### 6.5 Limitations and Scope

**Within a single compilation unit:** fully supported. Ownership tracking covers all
`append`/construction points, enum built incrementally.

**Cross-compilation-unit:** Relies on LTO (link-time optimization) to merge enum variants. `Animal`
as an existential type (`∃S. Animal(S)`) is passed across compilation unit boundaries. Each unit
generates partial enum variants, and the link phase merges them.

**Not supported:** runtime dynamic types (full duck typing). The type set is completely known at
compile time.

#### 6.6 Implementation Notes (Phase 3, v1 already landed)

The semantics of §6 (heterogeneous container, compile-time membership check, dispatch by actual
type, type set closed at compile time) have all been landed; the implementation has been concretized
at the mechanism layer as follows:

- **Type collection**: performs a one-shot collection of the implementation type set across the
  entire compilation unit via `ImplementationProof`, replacing the §6.2 "incremental collection at
  each ownership operation point". Within a single compilation unit the two are semantically
  equivalent (extra dead variants are harmless); the value of incremental collection lies in the
  cross-unit scenario, which falls into v2 (see below).
- **Representation**: the compiler synthesizes an `Animal$Group` variant type, a pure
  IR/bytecode/runtime artifact (instructions `CreateVariant`/`VariantTag`/`VariantPayload`, runtime
  value `RuntimeValue::Enum`); `MonoType` is unaware — at the typecheck layer the user-visible type
  is still the interface name. Every concrete value entering an existential-type position is
  automatically wrapped as a variant value (unified opaque representation, §6.4 semantics).
- **Wrapping points**: typecheck performs a targeted walkthrough at the "concrete vs. existential"
  judgment positions (annotated `let` / call argument / `return` / list literal element) and
  produces a span-keyed enforcement table; IR generation injects wrapping per span. Missing wrapping
  is loudly rejected by the runtime guard (`VariantTag`/`VariantPayload` requires the value to be a
  named group variant value); the worst case is an explicit runtime error during testing, never
  silently wrong data.
- **Dispatch**: variant-tag compare-and-jump chain; each arm statically calls the concrete method
  after unpacking the payload. RFC-004 rebinding form (`Type.method = fn[n]`) participates in
  dispatch as well after being reordered by binding position.
- **Isolation**: legacy trait constraints (`Drawable: Type = {..}` style, no generic parameters) do
  not pass through variant dispatch; behavior is unchanged.

**v1 boundary (subsequent phases)**: cross-unit LTO variant merging (§6.5); pattern matching on
Group values (depends on `match`'s IR support for variant patterns); reflection interaction;
Move-into-container semantics; `Any`/type-variable transit flow and inferred-lambda boundaries
(fallback = runtime guard).

---

## Use Case Analysis

### Basic Interface Implementation

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

# Usage
dog = Dog.new()
dog.speak()  # "Woof"
```

### Multiple Interface Implementation

```yaoxiang
# Multiple interfaces
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    name: (self: &Self) -> String,
}

# Type implementing multiple interfaces
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",
    name: (self: &Dog) -> String = "Buddy",
}

# Usage
dog = Dog.new()
dog.speak()  # "Woof"
dog.name()   # "Buddy"
```

### Generic Interface

```yaoxiang
# Generic interface
Container: (Self: Type, T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# Implement generic interface
IntList: Type = {
    data: Array(Int),
    Container(IntList, Int),
    add: (self: &mut IntList, item: Int) -> Void = ...,
    get: (self: &IntList, index: Int) -> Int = ...,
}
```

### Heterogeneous Container

```yaoxiang
# Interface definition
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# Type definition
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: &Cat) -> String = "Meow",
}

# Heterogeneous container
animals: List(Animal) = [Dog.new(), Cat.new()]

# Usage
for animal in animals {
    print(animal.speak())
}
# Output:
# Woof
# Meow
```

### Plugin System

```yaoxiang
# Interface definition
Plugin: (Self: Type) -> Type = {
    name: (self: &Self) -> String,
    execute: (self: &Self) -> Void,
}

# Main program
main: () -> Void = {
    # Load plugins
    plugin1 = load_plugin("plugin1.so")
    plugin2 = load_plugin("plugin2.so")

    # Compiler checks: plugin1 and plugin2 must implement the Plugin interface
    plugins: List(Plugin) = [plugin1, plugin2]

    # Execute all plugins
    for plugin in plugins {
        print(plugin.name())
        plugin.execute()
    }
}
```

---

## Trade-offs

### Advantages

1. **Concise**: no `impl` keyword needed
2. **Flexible**: method implementation supports both internal and external declarations
3. **Unified**: overload rules are consistent
4. **Convenient**: default value syntax is concise
5. **Zero overhead**: no vtable, compile-time type collection
6. **Type-safe**: interface matching is checked at compile time
7. **User-transparent**: no need to write `dyn Animal + 'a`

### Drawbacks

1. **Limitation**: no runtime dynamic types (full duck typing)
2. **Compile-time overhead**: must generate enum variants and `match` dispatch for each interface
3. **Type set**: must be completely known at compile time (within a single compilation unit)

### Mitigations

1. **Plugin system**: supported through compile-time interface match checking
2. **Type set**: ownership tracking, incremental construction — collect at each
   `append`/construction point, not a global scan
3. **Cross-compilation-unit**: link-time merging of enum variant sets, sharing mechanism with
   link-time monomorphization

---

## Alternatives

| Alternative            | Why not chosen                    |
| ---------------------- | --------------------------------- |
| `impl` keyword         | adds syntax complexity            |
| Vtable (`dyn Trait`)   | requires brand annotations (`'a`) |
| Full duck typing       | runtime overhead, not type-safe   |
| Enum wrapping (manual) | heavy burden on the user          |

---

## Relationship with RFC-009

**Brands and interface implementation**:

- Interface implementation lives at the type layer and does not involve brands
- Brands live at the borrow-proof layer (RFC-009a)
- The two are orthogonal and do not affect each other

**Dynamic dispatch and brands**:

- Dynamic dispatch uses implementation proof, no brand annotation needed
- Implementation proof is generated at compile time, with zero runtime lookup
- Avoids the complexity of `dyn Trait + 'a`

**Heterogeneous-container ownership**:

- Putting into `List(Animal)` is Move semantics (RFC-009); the original variable is no longer
  accessible
- Index access `animals[0]` returns `&AnimalGroup` (compiler-generated enum); the brand projection
  chain is `animals → animals[0] → enum_variant → field`
- Mutable access is implemented indirectly through interface methods, not exposing
  `&mut AnimalGroup` to users

## Interface Inheritance

Interfaces can include other interfaces. **No new syntax is introduced** — uses the exact same
syntax position as type declarations of interfaces:

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                       # Pet inherits Animal — no new keyword
    name: (self: &Self) -> String,
}

# When Dog implements Pet, it must satisfy all methods of both Animal and Pet
Dog: Type = {
    x: Int,
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",  # from Animal
    name: (self: &Dog) -> String = "Buddy",  # from Pet
}
```

**Design principle:** Inheritance exists but is not encouraged to be abused. The main composition
approach is through multiple interface instantiations
(`Dog: Type = { Animal(Dog), Pet(Dog), ... }`). A type can directly declare all interfaces it
satisfies, without needing to express that through an inheritance tree. Interface inheritance is
used only when there is a clear "is-a" hierarchy.

**Compiler processing:** expand the inheritance chain. `Pet(Self)` expands to
`{ all methods of Animal(Self), name: ... }`. When `Dog` declares `Pet(Dog)`, `Self ↦ Dog`; the
compiler verifies that `Dog` satisfies all methods of both `Animal(Dog)` and `Pet(Dog)`.

**`Self` substitution in interface inheritance**: in
`Pet: (Self: Type) -> Type = { Animal(Self), ... }`, the `Self` in `Animal(Self)` is `Pet`'s `Self`
parameter — it is substituted deferred. When `Dog` implements `Pet(Dog)`, `Self ↦ Dog`, and
`Animal(Self)` becomes `Animal(Dog)`. This is fully consistent with the parameter-passing semantics
of generic functions.

## Default Method Implementation

Interfaces can provide default method implementations. Implementing types may choose to override or
inherit the default implementation:

```yaoxiang
fmt: (Self: Type) -> Type = {
    display: (self: &Self) -> String,                      # must implement
    debug: (self: &Self) -> String = self.display(),       # ✅ references a same-interface method
    summary: (self: &Self) -> String = f"<{self.name}>",  # ❌ compile error: self.name is not in fmt
}
```

**Core constraint: interfaces cannot assume supertype implementations.** Default methods may only
reference methods already declared in the same interface. Concrete-type fields or other-interface
methods are not visible to default methods — an interface is a closed contract that cannot reach
into the implementer's pocket. Violating this constraint is **reported as an error at
interface-definition time**.

**Inheritance can assume subtype implementations:** when interface `Pet(Self)` inherits
`Animal(Self)`, `Pet`'s default methods may use the methods declared in `Animal` — because of
inheritance, they are guaranteed to exist.

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                                              # inheritance
    name: (self: &Self) -> String,
    introduce: (self: &Self) -> String = self.name() + " says " + self.speak(),  # ✅ speak comes from inherited Animal
}
```

**Compile-time behavior:** when a type implements an interface, for each method:

1. The type provides it → use the type's method
2. The type does not provide it, the interface has a default → the compiler inlines the default
   implementation into the type (zero vtable overhead)
3. The type does not provide it, the interface has no default → compile error

**Design principle:** default methods resemble the auto-derive mechanism of `Copy`/`Clone` — the
compiler auto-generates when needed, and the user may override. No `virtual` / `override` / `super`
keywords are introduced.

---

## Implementation Phases

| Phase    | Content                                                                       | Dependency |
| -------- | ----------------------------------------------------------------------------- | ---------- |
| Phase 1  | Interface declaration syntax (`(Self: Type) -> Type`) + `Self` type parameter | RFC-011    |
| Phase 2  | Interface instantiation (`Animal(Dog)`) + `Self ↦ ConcreteType` substitution  | Phase 1    |
| Phase 3  | Internal / external method declarations                                       | Phase 2    |
| Phase 4  | Overload and override rules                                                   | Phase 3    |
| Phase 5  | Default value syntax                                                          | Phase 3    |
| Phase 6  | Interface inheritance                                                         | Phase 4    |
| Phase 7  | Default method implementation                                                 | Phase 6    |
| Phase 8  | Implementation proof generation                                               | Phase 7    |
| Phase 9  | Compile-time type collection                                                  | Phase 8    |
| Phase 10 | Dynamic dispatch implementation                                               | Phase 9    |

---

## Design Decision Records

| Decision                          | Decision                                                                                                                     | Reason                                                                                                                                          | Date       |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Interface declaration syntax      | interface is a parameterized type `(Self: Type) -> Type`, instantiated on implementation                                     | Eliminates the `Self` magic keyword; fully consistent with the RFC-011 generics system                                                          | 2026-06-14 |
| `Self` type parameter             | explicit type parameter; syntax-checked at interface definition, fully checked at instantiation point                        | Avoids free type variables in HM inference                                                                                                      | 2026-06-14 |
| Dynamic dispatch                  | compile-time type collection + auto enum generation                                                                          | No vtable, zero runtime lookup, user-transparent                                                                                                | 2026-06-14 |
| External method declaration       | supported                                                                                                                    | Flexibility equivalent to internal declaration; compiler handles cross-file collection                                                          | 2026-06-14 |
| Override                          | forbidden (same signature errors)                                                                                            | Override causes unpredictable behavior; overload covers all cases                                                                               | 2026-06-14 |
| Interface inheritance             | supported, no new syntax                                                                                                     | Same syntax position as type-declared interfaces. Encourages composition (multiple interface instantiation), discourages deep inheritance trees | 2026-07-03 |
| Default method implementation     | supported, similar to `Copy`/`Clone` auto-derive                                                                             | Interface provides default body, compiler inlines on the implementing type; user may override. No `virtual`/`override` introduced               | 2026-07-03 |
| Default method constraint         | verified at interface-definition time: only same-interface methods may be referenced; cannot assume supertype implementation | Interface is a closed contract. Inheritance can assume subtype implementation, but interfaces cannot assume implementer-type fields/methods     | 2026-07-03 |
| Type collection strategy          | ownership tracking, incremental construction — collect at each `append`/construction point                                   | Not a global scan of all implementers, but incremental enum extension at each ownership operation point                                         | 2026-07-03 |
| `ImplementationProof`             | pure compile-time concept, erased at runtime                                                                                 | Runtime dispatches via enum `match`; the proof serves only for compile-time validation                                                          | 2026-07-03 |
| Cross-compilation-unit            | LTO merges enum variants                                                                                                     | Existential type is passed across compilation-unit boundaries; each unit generates partial enum, LTO phase merges                               | 2026-07-03 |
| Field / method namespace          | unified namespace, conflicts reported as errors                                                                              | Field access `point.x` and method call `point.x()` are syntactically indistinguishable; unification avoids ambiguity                            | 2026-07-03 |
| Heterogeneous-container ownership | Move semantics; original variable unusable after being placed into the container                                             | Consistent with the RFC-009 ownership model                                                                                                     | 2026-07-03 |
| Brand projection                  | `match` pattern binding produces sub-brands, equivalent to field projection                                                  | Consistent with the RFC-009a brand-tree mechanism; enum-variant projection is a legal path in the brand tree                                    | 2026-07-03 |
| Receiver spelling convention      | `&Self` borrow / `&mut Self` mutable borrow / by-value = Move                                                                | Receiver follows signature semantics (RFC-009); interface defaults to borrow; historical by-value spelling migrated to `&Self`                  | 2026-08-30 |

## Open Questions

- [x] ~~Interface inheritance (interfaces can inherit other interfaces)~~ → supported, no new
      syntax. `Pet: (Self: Type) -> Type = { Animal(Self), ... }`
- [x] ~~Default method implementation (interfaces can provide default implementations)~~ →
      supported, similar to `Copy` auto-derive. Interface provides body, compiler inlines on demand
- [x] ~~`Self` as an implicit magic keyword~~ → eliminated. `Self` is an explicit type parameter;
      the interface is `(Self: Type) -> Type`
- [ ] Advanced usage of interface constraints (associated types, GAT) — associated types realized
      via generic interface parameters (`Container: (Self: Type, T: Type) -> Type`); GAT requires
      further design
- [ ] Interaction with closures (closures implementing interfaces) — initial strategy: closures do
      not directly support implementing interfaces, a wrapper type is needed. Anonymous-type
      interface implementation deferred to a subsequent RFC

---

## References

- [RFC-011: Generic System Design](011-generic-type-system.md) — parent RFC
- [RFC-009: Ownership Model Design](009-ownership-model.md) — ownership system
- [RFC-009a: Borrow Proof Pipeline](009a-borrow-proof-pipeline.md) — brand mechanism
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — unified syntax

---

## Lifecycle and Destination

| Status       | Location                    | Description            |
| ------------ | --------------------------- | ---------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | formal design document |
