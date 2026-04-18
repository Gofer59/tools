cmake_minimum_required(VERSION 3.22.1)
project(whisper_jni LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(WHISPER_ROOT "${CMAKE_SOURCE_DIR}/../../../../third_party/whisper.cpp")

# whisper.cpp as subdirectory; its CMake builds a static "whisper" target.
set(WHISPER_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(WHISPER_BUILD_TESTS    OFF CACHE BOOL "" FORCE)
set(BUILD_SHARED_LIBS      OFF CACHE BOOL "" FORCE)
add_subdirectory(${WHISPER_ROOT} ${CMAKE_BINARY_DIR}/whisper.cpp)

add_library(whisper_jni SHARED
    whisper_jni.cpp
)

target_include_directories(whisper_jni PRIVATE
    ${WHISPER_ROOT}
    ${WHISPER_ROOT}/include
    ${WHISPER_ROOT}/ggml/include
)

target_link_libraries(whisper_jni
    whisper
    log
    android
)

# Android 15 16KB page-size compliance.
target_link_options(whisper_jni PRIVATE "-Wl,-z,max-page-size=16384")
