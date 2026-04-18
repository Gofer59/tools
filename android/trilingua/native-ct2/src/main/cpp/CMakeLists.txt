cmake_minimum_required(VERSION 3.22.1)
project(ct2_jni LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CT2_ROOT "${CMAKE_SOURCE_DIR}/../../../../third_party/CTranslate2")
set(SPM_ROOT "${CMAKE_SOURCE_DIR}/../../../../third_party/sentencepiece")

# CT2 CMake uses OPENMP_RUNTIME string (INTEL/COMP/NONE), not WITH_OPENMP bool.
set(OPENMP_RUNTIME      "NONE"  CACHE STRING "" FORCE)
# Correct CT2 option names (no CT2_ prefix):
set(WITH_MKL            OFF     CACHE BOOL "" FORCE)
set(WITH_CUDA           OFF     CACHE BOOL "" FORCE)
set(WITH_RUY            ON      CACHE BOOL "" FORCE)
set(WITH_DNNL           OFF     CACHE BOOL "" FORCE)
set(WITH_OPENBLAS       OFF     CACHE BOOL "" FORCE)
set(WITH_ACCELERATE     OFF     CACHE BOOL "" FORCE)
set(BUILD_CLI           OFF     CACHE BOOL "" FORCE)
set(BUILD_TESTS         OFF     CACHE BOOL "" FORCE)
set(BUILD_SHARED_LIBS   ON      CACHE BOOL "" FORCE)
set(ENABLE_CPU_DISPATCH OFF     CACHE BOOL "" FORCE)

# Vendor sentencepiece (required by ct2_jni.cpp for SPM tokenization).
# Disable all CLI tools/tests that fail to link on Android (no pthread sep. lib).
set(SPM_ENABLE_SHARED   OFF CACHE BOOL "" FORCE)
set(SPM_BUILD_TEST      OFF CACHE BOOL "" FORCE)
set(SPM_BUILD_EXAMPLES  OFF CACHE BOOL "" FORCE)
# Disable CLI binaries that depend on non-Android-compatible symbols
set(SPM_ENABLE_TCMALLOC OFF CACHE BOOL "" FORCE)
add_subdirectory(${SPM_ROOT} ${CMAKE_BINARY_DIR}/sentencepiece)
# Disable the CLI executables if they exist after add_subdirectory
foreach(spm_tool spm_train spm_encode spm_decode spm_normalize spm_export_vocab)
    if(TARGET ${spm_tool})
        set_target_properties(${spm_tool} PROPERTIES EXCLUDE_FROM_ALL TRUE)
    endif()
endforeach()

add_subdirectory(${CT2_ROOT} ${CMAKE_BINARY_DIR}/ctranslate2)

add_library(ct2_jni SHARED
    ct2_jni.cpp
)

target_include_directories(ct2_jni PRIVATE
    ${CT2_ROOT}/include
    ${CT2_ROOT}/src
    ${SPM_ROOT}/src
)

target_link_libraries(ct2_jni
    ctranslate2
    sentencepiece-static
    log
)

target_link_options(ct2_jni PRIVATE "-Wl,-z,max-page-size=16384")
