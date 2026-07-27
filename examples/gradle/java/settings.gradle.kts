rootProject.name = "example-gradle"

include("app", "lib")

dependencyResolutionManagement {
    repositories {
        mavenCentral()
    }
}
