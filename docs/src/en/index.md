---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: A programming language for the future
  tagline: All things proliferate; I observe their return
  actions:
    - theme: brand
      text: 🚀 Get Started
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
    description:
      'Philosophy of minimalism. From variables to functions, all declarations follow the name: type
      = value pattern, lower learning curve, more consistent code.'
    features:
      - Extreme uniformity in syntax declarations
      - Types are first-class citizens
  track02:
    rfc: RFC-011
    title: Zero-Cost Generics
    description:
      'Generic specialization is completed at compile time, type abstraction brings no runtime
      overhead. Compile-time monomorphization. Dead code elimination. Type system as macro.'
  track03:
    rfc: RFC-009
    title: Ownership Model
    description:
      'Farewell to GC stutters. YaoXiang uses a scope-based ownership model, memory safety is
      determined at compile time, no surprises.'
    features:
      - Shared references
      - Predictable
      - No GC stutters
      - No lifetimes
  track04:
    trackLabel: TRACK 04
    title: Decoupled Scheduler
    description:
      'From microcontrollers to high-performance servers, runtime adapts to the environment.
      Different scenarios choose different scheduling strategies, balancing performance and
      resources.'
    steps:
      - label: Embedded
        sub: 'Fully synchronous (Sync)'
      - label: Standard
        sub: 'Automated concurrency management based on DAG and lazy evaluation'
      - label: Full
        sub: 'Work-stealing mechanism'
  track05:
    title: Language Specification v1.8
    description:
      'No syntax sugar bombardment. 17 keywords cover all features, no complex syntax sugar, just
      pure expressiveness.'
---
