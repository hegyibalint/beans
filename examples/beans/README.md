# Scoping, in GOOD and BAD lines

Two projects, `a` and `b`, each with main and test code. A hand written
`beans.toml` describes them. Every line in the Java files is marked `// GOOD`
or `// BAD`, and the whole example exists to show where the line between them
falls.

The claim is: a file does not see the project it is in. It sees what its own
unit is allowed to see.

## Open it

```sh
scripts/dev-vscode.sh
```

Beans reads `beans.toml` at startup and loads all four files before you open
any of them, so F12 works into files you never touched.

## Four units, not two projects

```
a-main  <--  a-test
  ^
  |
b-main  <--  b-test
```

Main and test are separate units because they do not see the same things.
Arrows point at what a unit may look at.

## The matrix

One class per unit, named after it. Each class has four methods, one per
target, so all sixteen combinations are on screen. This is what the engine
actually answers:

| from ↓ / to → | `AMain` | `ATest` | `BMain` | `BTest` |
| --- | --- | --- | --- | --- |
| **a-main** | GOOD | BAD | BAD | BAD |
| **a-test** | GOOD | GOOD | BAD | BAD |
| **b-main** | GOOD | BAD | GOOD | BAD |
| **b-test** | GOOD | BAD | GOOD | GOOD |

Read the two test columns downwards. `ATest` is reachable from a-test and from
nowhere else. `BTest` is reachable from b-test and from nowhere else. That is
the whole rule about test code, in two columns:

- **Main cannot see its own tests.** `AMain.toOwnTests` reaches for `ATest`,
  which sits in the same project and in the same package `com.example.a`. It
  resolves to nothing. The package is not what decides this. The unit is.
- **Tests cannot see other tests.** `BTest.toOtherTests` reaches for `ATest`
  and gets nothing, even though b-test has the widest scope in the example.

Read the `BMain` column for the other direction: b depends on a, so a-main and
a-test both get BAD there. The dependency does not run backwards.

## Two axes, not one

Scoping decides whether a name resolves at all. JLS 26 §6.6.1 then decides
whether this place may touch what was found. The answers are independent, and
the files show both.

Look at `BMain.toOtherMain`. The edge to a-main exists, so all three names
resolve and F12 lands in `AMain.java`. Then:

```java
int a = target.open;    // GOOD, public
int b = target.shared;  // BAD, squiggle: package-private in com.example.a
int c = target.hidden;  // BAD, squiggle: private in AMain
```

Compare with `ATest.toOwnMain`, which reaches the same three fields of the same
class. There `shared` is GOOD, because a-test is also in `com.example.a`.

So the two kinds of BAD look different in the editor:

| kind | what you see |
| --- | --- |
| out of scope | `type-outside-scope` on the parameter type; F12 does nothing. |
| in scope, not accessible | `inaccessible-member` on the member; F12 still jumps to the declaration. |

The eight unavailable parameter types and six inaccessible member accesses make
fourteen squiggles in total. Member accesses such as `target.open` stay quiet
when `target` already has an out-of-scope type, avoiding a cascade.

## Edit beans.toml and watch it move

Remove `"a-main"` from `unit.b-test` and restart. `BTest.toOtherMain` goes
dark, all three lines, while `BMain.toOtherMain` keeps working. `depends_on`
does not chain, so an edge to `b-main` does not hand over what b-main depends
on.

## Turn the JDK on

`beans.toml` carries a commented-out `jdk_home`. Point it at a real JDK and
restart:

```sh
mise where java   # the JDK this repository pins
```

Beans then reads `<jdk_home>/lib/modules`, the runtime image holding every
system module, and puts all ~27,000 classes in that unit's scope. Add an import
to any file in the example and the type resolves:

```java
import java.util.List;

class AMain {
    void m(List target) {}   // GOOD once jdk_home is set, BAD without it
}
```

Two things to expect. The first load takes a few seconds in a debug build,
because the whole runtime is parsed up front. And F12 on `List` does nothing:
navigation lands on a declaration in a source file, and a class file has no
source to land in. What the JDK buys today is that the name resolves and stops
being squiggled.

`jdk_home` also works per unit, which is how a project whose units target
different releases would say so:

```toml
jdk_home = "/path/to/jdk-26"    # what every unit gets

[unit.legacy]
jdk_home = "/path/to/jdk-17"    # unless it says otherwise
```

## What works today

- `beans.toml` is read at startup, and every declared source is loaded before
  an editor asks for anything.
- One scope per unit, built by flattening `depends_on` once.
- Go to Definition and Go to Declaration for simple type names, fields and
  methods, across files and across units.
- Hover, but only for a declaration in the file you are hovering in.
- Three diagnostics: `type-outside-scope` (§6.5.5.1 and §7.3),
  `inaccessible-member` (§6.6.1), and `cannot-find-symbol` for a bare name that
  resolves to nothing.

## What does not work yet

Worth reading before the example confuses you.

- **No diagnostic for a genuinely unknown type.** `type-outside-scope` is
  deliberately narrower: it fires only when Beans has indexed a matching
  declaration and the current compilation cannot observe its source. Imports
  themselves are not diagnosed yet.
- **`java.lang` is not implicit.** A JDK can be read now, but reaching a type
  without naming it is stage 4 of `resolve_type_name` and unbuilt, so bare
  `String` resolves to nothing whether or not a JDK is loaded. Only a
  single-type import reaches one. See "Turn the JDK on" above.
- **Qualified type names.** `java.util.List` written inline is unresolved.
  Imports are walked, inline dotted names are not.
- **No inheritance.** `extends` is parsed and then not followed.
- **Only four statement forms are modeled**: blocks, local declarations,
  expression statements and `return`. Code inside `if`, `for`, `while` or `try`
  produces nothing, so nothing in there resolves or gets diagnosed. That is why
  every line in this example is a plain local declaration.
- **Operators are not expressions to us.** `a + b` contributes nothing, so a
  field access inside one is never checked.

## Files

```
beans.toml
a/src/main/java/com/example/a/AMain.java
a/src/test/java/com/example/a/ATest.java
b/src/main/java/com/example/b/BMain.java
b/src/test/java/com/example/b/BTest.java
```

The `examples/gradle` folder next to this one is a different thing: a real
Gradle build for the importer to read. This one is the hand written descriptor.
