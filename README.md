# Beans

Beans is a multi-language LSP for JVM languages. 
It targets Java, Kotlin, Groovy, Scala, and Clojure with a single shared index, so navigation, references, and refactors work across language boundaries instead of stopping at them.

## The killer feature: cross-language navigation

Real-world JVM projects mix languages constantly — Java with Kotlin (Android, Spring), Java with Groovy (Gradle, Spock), Scala with Java (data platforms). Every language boundary is a blind spot for separate LSPs:

- Renaming a Java interface method does not update its Kotlin implementation.
- Find-references on a Java class misses the Groovy test that calls it.
- Jumping from a Kotlin call site into the Java definition either fails or each LSP reimplements the other language's understanding from scratch.

Beans builds one symbol index that all five languages parse into. Go-to-definition, find-references, and (eventually) rename work across every JVM language in the project. No lightweight LSP outside IntelliJ does this today.
