---
title: Beans
description: An LSP for the JVM ecosystem.
template: doc
tableOfContents: false
hero:
  tagline: An LSP for the JVM ecosystem.
  image:
    file: ../../assets/logo.png
    alt: Beans logo
  actions:
    - text: Getting Started
      link: /beans/getting-started/
      icon: right-arrow
    - text: GitHub
      link: https://github.com/hegyibalint/beans
      icon: external
      variant: minimal
---

## What is Beans?

Beans aims to build a unified language server for as many JVM languages as possible,
using the Language Server Protocol (LSP) to bring that support to your editor.

### How is it different from other JVM language servers?

The JVM ecosystem already has established language servers:
[Eclipse JDT Language Server](https://github.com/eclipse-jdtls/eclipse.jdt.ls)
for Java, [Metals](https://scalameta.org/metals/) for Scala, and
[clojure-lsp](https://clojure-lsp.io/) for Clojure. There are also newer offerings,
including JetBrains' [Kotlin Language Server](https://github.com/Kotlin/kotlin-lsp)
and [Java and Kotlin by IntelliJ IDEA](https://marketplace.visualstudio.com/items?itemName=jetbrains.intellij-server),
which requires an IntelliJ IDEA Ultimate or All Products Pack subscription.
What makes Beans different?

JVM languages are built to work together. You can build a Kotlin core with a Java
public API, test its behavior in Groovy, and use Scala or Clojure where they fit.
Your tooling should follow those connections: navigation, diagnostics, and
refactoring should work across language boundaries, not stop at them.

Separate language servers can each understand their own language, but the LSP
does not define how they should share semantic information. Beans explores moving
up a level of abstraction: a shared language-processing and indexing engine that
uses the JVM as a common denominator, with a unified language server on top.

### That sounds like a lot of work.

Indeed. Building this language support takes a lot of time, effort and care.

Beans is looking for people who believe in this mission.
Interested in joining and helping? Read and ask in
[GitHub Discussions](https://github.com/hegyibalint/beans/discussions), and see
[CONTRIBUTING](https://github.com/hegyibalint/beans/blob/main/CONTRIBUTING.md)
to understand more about how we work.

### What is your plan?

Beans is in pre-alpha, with three foundations taking shape at the same time:

1. **The core and engine:** shared language abstractions, workspace and query
   handling, event processing, and persistent storage—the infrastructure that
   language support and tools build on.
2. **The JVM platform:** class-file and container parsing, plus the JVM-level
   types and semantics that language implementations can share. This is the
   common foundation for the languages Beans will support.
3. **Java support:** parsing `.java` files and building the first language
   implementation to exercise the core, engine, and JVM platform together.

This is Stage 1 of Beans: proving the foundations with Java before extending
support to other JVM languages. Features are incomplete, APIs are evolving, and
Beans is not yet a replacement for your everyday language server.

## Cool. How do I try it out?

Head to [Getting Started](/beans/getting-started/) to build Beans from source and
try it in your editor. Prebuilt binaries are not available yet.
