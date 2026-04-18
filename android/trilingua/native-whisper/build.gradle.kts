plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "com.trilingua.app.nativebridge.whisper"
    compileSdk = 34
    ndkVersion = "27.1.12297006"

    defaultConfig {
        minSdk = 26
        ndk { abiFilters += listOf("arm64-v8a") }
        externalNativeBuild {
            cmake {
                cppFlags += listOf("-std=c++17", "-O3", "-DNDEBUG")
                cFlags += listOf("-O3", "-DNDEBUG")
                arguments += listOf(
                    "-DANDROID_STL=c++_shared",
                    "-DCMAKE_BUILD_TYPE=Release",
                    "-DCMAKE_SHARED_LINKER_FLAGS=-Wl,-z,max-page-size=16384",
                    "-DWHISPER_BUILD_EXAMPLES=OFF",
                    "-DWHISPER_BUILD_TESTS=OFF",
                    "-DGGML_OPENMP=OFF"
                )
            }
        }
    }

    // Force Release optimization for native libs regardless of Java build type.
    // Debug-mode native builds run whisper ~100× slower than release.
    buildTypes {
        debug {
            externalNativeBuild {
                cmake {
                    arguments += "-DCMAKE_BUILD_TYPE=Release"
                }
            }
        }
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    implementation(libs.kotlinx.coroutines.core)
}
