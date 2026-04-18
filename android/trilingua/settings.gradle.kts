pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        // sherpa-onnx publishes to Maven Central; no custom repo needed.
    }
    // gradle/libs.versions.toml is auto-discovered by Gradle 7.4+; no explicit registration needed.
}
rootProject.name = "Trilingua"
include(":app", ":native-whisper", ":native-ct2")
