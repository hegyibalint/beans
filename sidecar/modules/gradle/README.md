# Gradle source-set model

The adapter queries a small custom Tooling API model rather than projecting the
IDEA model's main/test dependency scopes. It imports the requested build's
projects; included builds and `buildSrc` are not recursively imported.

```text
gradle/model/              Provider and serializable snapshots (Java 8 bytecode)
gradle/src/main/           Sidecar module and shared Module translation
```

An init script registers a project-local builder. Only the provider JAR is added
to the init-script classpath, not the sidecar assembly. The lifecycle hook requires
Gradle 8.8+. The model has been exercised against Gradle 9.6.1 and 9.9.0.

Each project reports its directory, default JVM toolchain home (if available),
source sets, and JVM features. Each source set reports:

1. Its name and declared `allSource` directories, including missing generated roots.
2. Its currently selected `allSource` files, including resources and languages
   registered by plugins, with their source-set filters applied.
3. Its compile classpath and class-output directories, without requiring them to exist.

The provider has no Kotlin or Groovy plugin dependencies. The contract is Gradle's
`SourceSet`, not an assumption about compiler task types or source-set names.
Kotlin Multiplatform's separate source-set hierarchy is not covered by that claim.
Compiler-task classpath overrides, per-task toolchains, compiler flags and module
paths are outside this common model.

## JVM feature snapshots

Each feature reports its name, the name of its source set, and the artifact paths
published by its API/runtime configurations, including secondary artifact variants.
Paths are normalized and deduplicated; JARs and output directories need not exist.
The source-set link is read from the feature, not inferred from task names or paths.
Source sets without a published JVM feature remain in the source-set snapshot.

Extraction uses Gradle's internal JVM component/feature API, isolated in
`JvmFeatures`. A small reflective boundary handles configuration getters returning
either configurations or providers across Gradle versions. This API is not a
Gradle compatibility guarantee; integration tests cover the exercised versions.
Extraction reads existing API/runtime publications without enabling source or
Javadoc variants.

## Module translation

`gradle/import` returns one shared `Module` per project with JVM source sets.
Each source set contributes a `Classpath` and an `ArtifactSources` association.
Inputs and associated sources are exact selected-file filters; declared roots are
not widened into unrestricted directory filters. New or generated files need
another import before they become registered inputs.

Classpath entries contain the compile classpath and the project's default JDK
runtime. Artifact associations contain the source set's class directories and
its features' declared outgoing roots, including JARs. External binary dependencies
without known source attachments remain ordinary classpath entries.

Engine resolves candidate visibility through shared module classpaths and
artifact/source associations, including across independent import snapshots.
It never follows a producer's compilation dependencies to widen consumer visibility.
The adapter retains actual compile artifacts rather than injecting producer
source files into classpaths. Both JAR and class-directory consumers reach sources
through artifact associations, preserving the artifact that admitted them.

Import requests do not schedule compilation or generation tasks. Gradle still
configures the build and may compile build logic or resolve/download dependencies
as part of configuration. Automatic JDK provisioning is disabled. Engine
registration atomically stores complete module snapshots per import kind/root;
it does not scan or index these resources.

## Validation

`./gradlew integrationTest` covers Java/Kotlin/Groovy application, test and custom
source sets, source filtering, missing generated roots, and compile-versus-runtime
visibility. Feature tests cover source-set associations, custom features, test
fixtures, configured/replaced outgoing JARs, secondary variants, and project
consumers selecting either JARs or class directories without compilation.
The mixed-language fixture resolves Kotlin JVM plugin 2.4.21 on its first run;
it does not add Kotlin to the provider's dependencies.

JUnit Framework was cloned for a one-off probe, not added as a permanent fixture
or test dependency.

## Editor behavior and limits

Java field-type navigation and resolved hover use the same classpath-aware lookup.
Hover distinguishes the declaration's source location from the artifact through
which it is reachable (`Via`). Neither lookup requires that artifact to exist.
Method-parameter navigation and virtual documents for compiled classes are not yet
implemented. Distinct locator–provenance pairs remain ambiguous; current navigation
does not expose multiple-definition results.

The module schema replaces the old compilation-scope cache. Updating resets older
caches; re-run the Gradle import to register this build's metadata again.
