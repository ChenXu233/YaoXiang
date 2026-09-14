---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: A programming language for the future
  tagline: All things arise together; I observe their return
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
    title: 'Unified Syntax'
    description:
      'Minimalist philosophy. From variables to functions, all declarations follow the name: type =
      value pattern, lowering the learning curve and keeping code consistent.'
    features:
      - Declarations are extremely uniform
      - Types are first-class citizens
  track02:
    rfc: RFC-011
    title: 'Zero-Cost Generics'
    description:
      'Generic specialization happens at compile-time, with no runtime overhead from type
      abstraction. Compile-time monomorphization. Dead code elimination. Type system as macros.'
  track03:
    rfc: RFC-009
    title: 'Ownership Model'
    description:
      'Goodbye to GC pauses. YaoXiang uses a scope-based ownership model, with memory safety
      guaranteed at compile-time — no surprises.'
    features:
      - Shared references
      - Predictable
      - No GC pauses
      - No lifetimes
  track04:
    trackLabel: TRACK 04
    title: 'Decoupled Scheduler'
    description:
      'From microcontrollers to high-performance servers, the runtime adapts to the environment.
      Different scenarios choose different scheduling strategies, achieving both performance and
      resource efficiency.'
    steps:
      - label: Embedded
        sub: 'Fully synchronous (Sync)'
      - label: Standard
        sub:
          'Automated concurrency management based on Directed Acyclic Graph (DAG) and lazy
          evaluation'
      - label: Full
        sub: 'Work-stealing mechanism (WorkSteal)'
  track05:
    title: 'Language Specification v1.8'
    description:
      'No syntax sugar bombardment. 17 keywords cover all features — no complex syntax sugar, only
      pure expressiveness.'
---
