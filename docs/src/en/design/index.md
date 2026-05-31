# YaoXiang Design Document

> The Tao gives birth to one, one gives birth to two, two gives birth to three, three gives birth to all things.

This directory contains design decisions, proposals, and discussions for the YaoXiang programming language.

## Core Design Philosophy

| Philosophy | Description |
|------------|-------------|
| **Everything is a Type** | Values, functions, and modules are all types; types are first-class citizens |
| **Natural Syntax** | Python-like readability, close to natural language |
| **Ownership Model** | Zero-cost abstractions, no GC, high performance |
| **Spawn Model** | Synchronous syntax, asynchronous nature, automatic parallelism |
| **AI-Friendly** | Strictly structured, clear AST |

## Design Document Structure

```
design/
├── index.md              # This index
├── accepted/             # Accepted design proposals
│   └── *.md
├── rfc/                  # RFC proposals (pending review)
│   ├── *.md
│   └── RFC_TEMPLATE.md
└── discussion/           # Design discussion area (open discussion)
    └── *.md
```

## Accepted Design Proposals

| Document | Status | Description |
|----------|--------|-------------|
| [008-Concurrency Model](./accepted/008-runtime-concurrency-model.md) | ✅ Official | Spawn model and task scheduler design |

> See the [`accepted/`](./accepted/) directory for the complete list.

## RFC Proposals

> RFC (Request for Comments) is the proposal process for new features and significant changes.

### Active Proposals

| Number | Title | Status |
|--------|-------|--------|
| RFC-003 | Version Planning | Pending Review |
| RFC-005 | Automated CVE Scanning | Pending Review |
| RFC-006 | Documentation Site Optimization | Pending Review |
| RFC-012 | f-string Template Strings | Pending Review |

### RFC Template

Before submitting a new proposal, please refer to:
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [Full Example](./rfc/EXAMPLE_full_feature_proposal.md)

## Participating in Design Discussions

### Proposal Process

```
1. Draft proposal (use RFC template)
   → Place in rfc/ directory

2. Community discussion
   → Discuss in the corresponding issue in rfc/REPO

3. Core team review
   → Accepted → Move to accepted/
   → Rejected → Move to archived/ or delete
```

### Design Principles

- **Clear Boundaries**: Each design decision should have a clear scope of application
- **Practicality First**: Solve real problems, not hypothetical threats
- **Progressive Transparency**: Layered design of concurrency model (L1-L3)
- **User-Visible Behavior Invariant**: Never break userspace

## Code Examples

```yaoxiang
// Type definition
Point: Type = { x: Float, y: Float }
Result: Type[T, E] = { ok(T) | err(E) }

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Spawn function (automatic concurrency)
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

// Main function
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## Key Design Decisions

### 1. Type System

- **Unified Type Syntax**: Abolish `enum`, `struct`, `union`, and unify with `Name: Type = {...}`
- **Constructors are Types**: Bridge the gap between "type" and "value"
- **Generics Support**: Compile-time monomorphization, zero runtime overhead

### 2. Spawn Model

```yaoxiang
// Three layers of concurrency abstraction

// L1: @blocking synchronous (disable parallelism)
fetch: (String) -> JSON @blocking = (url) => { ... }

// L2: spawn explicit concurrency
process: () -> Void spawn = () => {
    users = fetch_users()
    posts = fetch_posts()  // Automatic parallelism
}

// L3: Fully transparent (default)
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)  // System automatically analyzes dependencies
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3. Error Handling

```yaoxiang
Result: Type[T, E] = { ok(T) | err(E) }

process: () -> Result[Data, Error] = {
    data = fetch_data()?      // ? operator propagates transparently
    transformed = transform(data)?
    save(transformed)?
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn to use YaoXiang
- [Reference Documentation](../reference/) - API and standard library
- [Language Specification](../reference/language-spec/index.md) - Complete language specification
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [Contributing Guide](../tutorial/contributing.md)

## Historical Archive

Historical documents from the design process have been moved to the [`docs/old/`](../../old/) directory, including:
- Early architecture designs
- Abandoned proposals
- Outdated implementation plans