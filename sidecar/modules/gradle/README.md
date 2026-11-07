# Gradle source-set model

The adapter queries a small custom Tooling API model rather than projecting the
IDEA model's main/test dependency scopes. It imports the requested build's
projects; included builds and `buildSrc` are not recursively imported.

```text
gradle/model/              Provider and serializable snapshots (Java 8 bytecode)
gradle/src/main/           Sidecar module and CompilationScope translation
```

An init script registers a project-local builder. Only the provider JAR is added
to the init-script classpath, not the sidecar assembly. The lifecycle hook requires
Gradle 8.8+. The model has been exercised against Gradle 9.6.1 and 9.9.0.

Each project reports its directory, default JVM toolchain home (if available),
and source sets. Each source set reports:

1. Its name and declared `allSource` directories, including missing generated roots.
2. Its currently selected `allSource` files, including resources and languages
   registered by plugins, with their source-set filters applied.
3. Its compile classpath and class-output directories, without requiring them to exist.

The provider has no Kotlin or Groovy plugin dependencies. The contract is Gradle's
`SourceSet`, not an assumption about compiler task types or source-set names.
Kotlin Multiplatform's separate source-set hierarchy is not covered by that claim.
Compiler-task classpath overrides, per-task toolchains, compiler flags and module
paths are outside this common model.

## Scope translation

Inputs are exact selected-file filters; declared roots are retained in the tooling
model but are not widened into unrestricted directory filters. New or generated
files need another import before they become registered inputs.

Dependencies contain the compile classpath and the project's default JDK runtime.
A classpath entry matching a known class-output directory also admits that source
set's inputs. Arbitrary JARs remain binary filters: the adapter does not guess which
source sets they package. This is not yet a general source-attachment model.

Import requests do not schedule compilation or generation tasks. Gradle still
configures the build and may compile build logic or resolve/download dependencies
as part of configuration. Automatic JDK provisioning is disabled. Engine
registration persists metadata only; it does not scan or index these resources.

## Validation

`./gradlew integrationTest` covers Java/Kotlin/Groovy application, test and custom
source sets, source filtering, missing generated roots, and compile-versus-runtime
visibility. The mixed-language fixture resolves Kotlin JVM plugin 2.4.21 on its
first run; it does not add Kotlin to the provider's dependencies.

JUnit Framework was cloned for a one-off probe, not added as a permanent fixture
or test dependency.
