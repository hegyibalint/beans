rootProject.name = "sidecar"

include("core", "adapter-gradle", "app")

rootProject.children.forEach { it.projectDir = file("subprojects/${it.name}") }

dependencyResolutionManagement {
    // :app assembles the fat jar, so it resolves the adapters' transitive
    // dependencies too. Declaring repositories per project would leave it
    // looking for the Tooling API somewhere that does not publish it.
    repositoriesMode = RepositoriesMode.FAIL_ON_PROJECT_REPOS
    repositories {
        mavenCentral()
        maven(url = "https://repo.gradle.org/gradle/libs-releases")
    }
}
