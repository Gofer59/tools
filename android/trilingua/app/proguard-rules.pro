# Native bridge classes — must keep fully-qualified names and native methods.
-keep class com.trilingua.app.nativebridge.** { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}

# Sherpa-onnx: avoid stripping JNI entry points.
-keep class com.k2fsa.sherpa.onnx.** { *; }

# Keep data classes used across Kotlin reflection boundaries (none today, but safe).
-keepclassmembers class com.trilingua.app.model.** { *; }
