---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: A Programming Language for the Future
  tagline: 万物并作，吾以观复
  actions:
    - theme: brand
      text: 🚀 Quick Start
      link: /en/getting-started
    - theme: alt
      text: Tutorials
      link: /en/tutorial/
    - theme: brand
      text: Download
      link: /en/download/
    - theme: alt
      text: GitHub ⇗
      link: https://github.com/ChenXu233/yaoxiang

tracks:
  track01:
    trackLabel: TRACK 01
    rfc: RFC-010
    title: Unified Syntax
    description: 'The philosophy of minimalism. Everything follows the `name: type = value` pattern.'
    features:
      - Consistent Declaration
      - First-class Functions
      - Type The Universe
  track02:
    rfc: RFC-011
    title: Zero-Cost Generics
    description: Compile-time monomorphization means your abstractions cost nothing at runtime. The type system acts as a powerful macro engine.
  track03:
    rfc: RFC-009
    title: Ownership Model
    description: Memory safety without the garbage collector pauses. YaoXiang uses a refined ownership model that simplifies lifetime management based on scope.
    features:
      - Ref Sharing
      - Predictable
      - GC Pauses
      - Lifetimes
  track04:
    trackLabel: TRACK 04
    title: Decoupled Scheduler
    description: From microcontrollers to high-performance servers. The runtime adapts to the environment.
    steps:
      - label: Embedded
        sub: "(Sync)"
      - label: Standard
        sub: "(DAG)"
      - label: Full
        sub: "(WorkSteal)"
  track05:
    title: Language Spec v1.6
    description: Pure expression without syntax sugar overload. A minimal surface area for maximum capability.

---
