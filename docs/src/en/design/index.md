# YaoXiang Design Documents

> 道生一，一生二，二生三，三生万物。

This directory contains design decisions, proposals, and discussions for the YaoXiang programming language.

## Core Design Principles

| Principle           | Description                                               |
| ------------------- | --------------------------------------------------------- |
| **Everything is a type** | Values, functions, modules are all types; types are first-class citizens |
| **Natural syntax** | Python-like readability, close to natural language        |
| **Ownership model** | Zero-cost abstraction, no GC, high performance           |
| **Spawn model**     | Synchronous syntax, asynchronous nature, automatic parallelism |
| **AI-friendly**     | Strictly structured, clear AST                            |

## Design Document Structure

```
design/
├── index.md              # This index
├── deprecated/           # Deprecated (replaced by new design)
│   └── *.md
├── rejected/             # Rejected
│   └── *.md
├── rfc/
│   ├── draft/            # Draft (work in progress)
│   ├── review/           # Under review (open for discussion)
│   ├── accepted/         # Accepted (design approved)
│   ├── deprecated/       # Deprecated (replaced)
│   └── rejected/         # Rejected (not approved)
└── discussion/           # Design discussion area (open for discussion)
    └── *.md
```

## Accepted Design Proposals

| Document                                                                                                             | Status      | Description                 |
| -------------------------------------------------------------------------------------------------------------------- | ----------- | --------------------------- |
| [RFC-010 Unified Type Syntax](./rfc/accepted/010-unified-type-syntax.md)                                            | ✅ Accepted | Unified type definition syntax |
| [RFC-011 Generic Type System](./rfc/accepted/011-generic-type-system.md)                                            | ✅ Accepted | Generic type system design  |
| [RFC-009 Ownership Model](./rfc/accepted/009-ownership-model.md)                                                    | ✅ Accepted | Ownership and borrowing system design |
| [RFC-024 Concurrency Model](./rfc/accepted/024-concurrency-model.md)                                                | ✅ Accepted | Spawn concurrency primitive semantics |
| [RFC-027 Compile-time Assertions](./rfc/accepted/027-compile-time-evaluation-types.md)                              | ✅ Accepted | Compile-time predicates and static verification |

> See the [`rfc/accepted/`](./rfc/) directory for the complete list (16 total), and [`rfc/index.md`](./rfc/index.md) for the latest status.

## RFC Proposals

> RFC (Request for Comments) is the proposal process for new features and major changes.

### Active Proposals

| Number   | Title                    | Status     |
| -------- | ------------------------ | ---------- |
| RFC-019  | Typed Hodosiimorphism    | Draft      |
| RFC-028  | JIT Compiler             | Draft      |
| RFC-029  | Module Semantic System   | Draft      |
| RFC-031  | Optimization Levels      | Draft      |
| RFC-033  | Reflection Operators     | Draft      |
| RFC-034  | Debugging Toolchain      | Draft      |
| RFC-035  | MCP Server               | Draft      |
| RFC-002  | Cross-platform IO (libuv) | Draft     |
| RFC-026b | yx-bindgen               | Draft      |
| RFC-011a | Interface Implementation and Dynamic Dispatch | Under Review |
| RFC-014a | Registry Protocol        | Under Review |
| RFC-014b | Build System             | Under Review |
| RFC-014c | Workspaces               | Under Review |
| RFC-026a | Extensible FFI           | Under Review |
| RFC-032  | Spawn Unified Expression | Under Review |

### Accepted Proposals

| Number   | Title                      | Status     |
| -------- | -------------------------- | ---------- |
| RFC-004  | Curried Multi-position Binding | Accepted |
| RFC-006  | Documentation Site Optimization | Accepted |
| RFC-007  | Unified Function Syntax    | Accepted   |
| RFC-008  | Runtime Concurrency Model  | Accepted   |
| RFC-009  | Ownership Model            | Accepted   |
| RFC-009a | Token Lifetime Analysis    | Accepted   |
| RFC-010  | Unified Type Syntax        | Accepted   |
| RFC-011  | Generic System             | Accepted   |
| RFC-012  | f-string                   | Accepted   |
| RFC-013  | Error Code Specification   | Accepted   |
| RFC-014  | Package Manager            | Accepted   |
| RFC-015  | Configuration System       | Accepted   |
| RFC-017  | LSP Support                | Accepted   |
| RFC-018  | LLVM AOT Compiler          | Accepted   |
| RFC-024  | Concurrency Model          | Accepted   |
| RFC-026  | FFI Core Mechanism         | Accepted   |
| RFC-027  | Compile-time Assertions     | Accepted   |
| RFC-030  | assert Mechanism           | Accepted   |

### Rejected Proposals

| Number   | Title               | Status     |
| -------- | ------------------- | ---------- |
| RFC-003  | Version Planning    | Rejected   |
| RFC-005  | CVE Scanning        | Rejected   |
| RFC-016  | Quantum Native Support | Rejected |
| RFC-025  | Primitive Type Extension | Rejected |

### RFC Template

Before submitting a new proposal, please refer to:

- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [Full Example](./rfc/EXAMPLE_full_feature_proposal.md)

## Participating in Design Discussions

### RFC Lifecycle

RFC proposals have 5 statuses:

| Status       | Meaning                                   |
| ------------ | ----------------------------------------- |
| Draft        | Work in progress                          |
| Under Review | Open for discussion                       |
| Accepted     | Design approved                           |
| Deprecated   | Once accepted, replaced by new design     |
| Rejected     | Not approved                              |

Full lifecycle:

```
Draft → Under Review → Accepted → Deprecated (replaced)
                          ↓
                      Rejected (not approved)
```

### Proposal Process

```
1. Draft proposal (use RFC template)
   → Place in rfc/draft/

2. Submit for review
   → Move to rfc/review/, open community discussion

3. Core team review
   → Accept → Move to rfc/accepted/
   → Reject → Move to rfc/rejected/

4. Ongoing maintenance
   → Superseded → Move to rfc/deprecated/
```

### Design Principles

- **Clear boundaries**: Each design decision should have a clear scope of application
- **Practicality first**: Solve real problems, not imagined threats
- **User-visible behavior unchanged**: Never break userspace

## Code Examples

```yaoxiang
// Type definition
Point: Type = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Main function
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## Key Design Decisions

### 1. Type System

- **Unified type syntax**: Abolish `enum`, `struct`, `union`, unified using `Name: Type = {...}`
- **Constructors are types**: Bridge the gap between "types" and "values"
- **Generics support**: Compile-time monomorphization, zero runtime overhead

### 2. Spawn Model

```yaoxiang
// Spawn model: sequential execution by default, spawn introduces dataflow parallelism

// Sequential execution by default
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)  // Sequential, wait for a to complete
    c = heavy_calc(3)  // Sequential, wait for b to complete
    a + b + c
}

// Spawn block introduces dataflow parallelism
process: () -> Void = () => {
    spawn {
        users = fetch_users()   // Parallel
        posts = fetch_posts()   // Parallel
    }
    // Caller synchronously blocks waiting for results
    render(users, posts)
}
```

### 3. Error Handling

```yaoxiang
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

process: () -> Result(Data, Error) = {
    data = fetch_data()?      // ? operator transparently propagates
    transformed = transform(data)?
    save(transformed)?
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn to use YaoXiang
- [Reference](../reference/) - API and standard library
- [Language Specification](../reference/language-spec/index.md) - Complete language specification
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [Contributing Guide](../dev/contributing.md)

## Historical Archive

Historical documents from the design process have been moved to the `docs/src/archive/` directory (this directory is not included in site building), including:

- Early architecture design
- Deprecated proposals
- Outdated implementation plans