---
title: Beans
description: An LSP for the JVM ecosystem.
template: doc
tableOfContents: false
hero:
  title: '<span class="beans-wordmark">Beans</span>'
  tagline: An LSP for the JVM ecosystem.
  image:
    file: ../../assets/logo.svg
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
and [Java and Kotlin by IntelliJ IDEA](https://marketplace.visualstudio.com/items?itemName=jetbrains.intellij-server) (given you have an Ultimate subscription).
What makes Beans different?

Languages in the JVM ecosystem can work together very well.
You can build a Kotlin core with a Java public API, test its behavior in Groovy, and use Scala or Clojure where they fit.
And you should expect the same from your tooling: navigation, diagnostics, and refactoring should work across languages just as well!

Separate language servers can each understand their own language (and often Java), but not their other siblings, because LSPs are not built to talk to each other.
Beans is based on the realization that a potential solution is a single, universal LSP for the JVM ecosystem.

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

To try out Beans as it stands today, see the project [README.md](https://github.com/hegyibalint/beans/blob/main/README.md#how-to-beans).
