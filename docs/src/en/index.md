```yaml
---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang
  text: A Programming Language for the Future
  tagline: All things flourish, I observe their return
  actions:
    - theme: brand
      text: 🚀 Quick Start
      link: /tutorial/getting-started
    - theme: alt
      text: Tutorial
      link: /tutorial/
    - theme: brand
      text: Download
      link: /download
    - theme: alt
      text: GitHub ⇗
      link: https://github.com/ChenXu233/yaoxiang

tracks:
  track01:
    trackLabel: TRACK 01
    rfc: RFC-010
    title: Unified Syntax
    description: "Minimalist philosophy. From variables to functions, all declarations follow the `name: type = value` pattern — lower learning cost, more consistent code."
    features:
      - Extremely unified syntax declarations
      - Types are first-class citizens
  track02:
    rfc: RFC-011
    title: Zero-Cost Generics
    description: "Generic specialization happens at compile-time, type abstraction brings no runtime overhead. Compile-time monomorphization. Dead code elimination. The type system is the macro."
  track03:
    rfc: RFC-009
    title: Ownership Model
    description: "Say goodbye to GC pauses. YaoXiang uses a scope-based ownership model — memory safety is determined at compile-time, no surprises."
    features:
      - Shared references
      - Predictable
      - No GC pauses
      - No lifetimes
  track04:
    trackLabel: TRACK 04
    title: Decoupled Scheduler
    description: "From microcontrollers to high-performance servers, the runtime adapts to the environment. Different scenarios choose different scheduling strategies, balancing performance and resources."
    steps:
      - label: Embedded
        sub: "Fully synchronous (Sync)"
      - label: Standard
        sub: "Automated concurrency management based on Directed Acyclic Graph (DAG) and lazy evaluation"
      - label: Full
        sub: "Work stealing mechanism (WorkSteal)"
  track05:
    title: Language Specification v1.8
    description: "Reject syntax sugar bombing. 17 keywords cover all features — no complex syntax sugar, only pure expressiveness."

---
```