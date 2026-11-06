rootProject.name = "sidecar"

include(":core", ":gradle", ":stdout-screamer")

project(":gradle").projectDir = file("modules/gradle")
project(":stdout-screamer").projectDir = file("fixtures/stdout-screamer")

dependencyResolutionManagement {
    // The root assembles the fat jar, so it resolves the adapters' transitive
    // dependencies too. Declaring repositories per project would leave it
    // looking for the Tooling API somewhere that does not publish it.
    repositoriesMode = RepositoriesMode.FAIL_ON_PROJECT_REPOS
    repositories {
        mavenCentral()
        maven(url = "https://repo.gradle.org/gradle/libs-releases")
    }
}
