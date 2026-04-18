plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "com.trilingua.app.nativebridge.ct2"
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
                    // CT2 CMake uses OPENMP_RUNTIME string (INTEL/COMP/NONE), not WITH_OPENMP bool.
                    "-DOPENMP_RUNTIME=NONE",
                    // Correct CT2 cmake option names (no CT2_ prefix for backend options):
                    "-DWITH_MKL=OFF",
                    "-DWITH_CUDA=OFF",
                    "-DWITH_RUY=ON",
                    "-DWITH_DNNL=OFF",
                    "-DWITH_OPENBLAS=OFF",
                    "-DWITH_ACCELERATE=OFF",
                    "-DBUILD_CLI=OFF",
                    "-DBUILD_TESTS=OFF",
                    // Disable ISA dispatch — not needed for single-ABI Android build
                    "-DENABLE_CPU_DISPATCH=OFF"
                )
            }
        }
    }

    // Force Release optimization for native libs regardless of Java build type.
    // Debug-mode ct2 + whisper builds run ~100× slower than release.
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
