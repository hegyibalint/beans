# Clojure Language Reference

**Source:** https://github.com/clojure/clojure-site
**Format:** AsciiDoc (.adoc)

## Core Language

- [Reader](reader.adoc) — Clojure reader, syntax, literal forms
- [Evaluation](evaluation.adoc) — Evaluation model, special forms overview
- [Special Forms](special_forms.adoc) — def, if, do, let, quote, fn, loop/recur, etc.
- [Vars](vars.adoc) — Var model, dynamic binding, interning
- [Namespaces](namespaces.adoc) — Namespace system, require, use, import
- [Macros](macros.adoc) — Macro writing and expansion
- [Metadata](metadata.adoc) — Metadata on symbols, collections, vars
- [Compilation](compilation.adoc) — AOT compilation

## Data Structures

- [Data Structures](data_structures.adoc) — Lists, vectors, maps, sets, core collection abstractions
- [Sequences](sequences.adoc) — Seq abstraction, sequence library
- [Transients](transients.adoc) — Mutable performance optimization for collections
- [Lazy Evaluation](lazy.adoc) — Lazy sequences

## Functions & Polymorphism

- [Multimethods](multimethods.adoc) — Multimethod dispatch (defmulti/defmethod)
- [Protocols](protocols.adoc) — Protocol-based polymorphism (defprotocol)
- [Datatypes](datatypes.adoc) — deftype, defrecord, reify
- [Other Functions](other_functions.adoc) — Miscellaneous core functions

## Concurrency & State

- [Refs](refs.adoc) — Refs and STM (Software Transactional Memory)
- [Atoms](atoms.adoc) — Atomic state management
- [Agents](agents.adoc) — Asynchronous agents
- [Reducers](reducers.adoc) — Parallel reduce/fold framework
- [Transducers](transducers.adoc) — Composable algorithmic transformations

## Java Interop

- [Java Interop](java_interop.adoc) — Calling Java from Clojure, type hints, proxy, gen-class

## Tooling & Infrastructure

- [Clojure CLI](clojure_cli.adoc) — Clojure CLI tools overview
- [Deps and CLI](deps_and_cli.adoc) — deps.edn and CLI reference
- [deps.edn](deps_edn.adoc) — deps.edn format reference
- [Dep Expansion](dep_expansion.adoc) — Dependency expansion algorithm
- [Libs](libs.adoc) — Library conventions
- [REPL and Main](repl_and_main.adoc) — REPL, main entry points
- [Other Libraries](other_libraries.adoc) — Additional bundled libraries

## Background

- [Lisps](lisps.adoc) — Clojure's relationship to other Lisps
- [Documentation](documentation.adoc) — Documentation conventions

## Guides

- [spec Guide](guides-spec.adoc) — clojure.spec guide (validation, conformance, generation)
