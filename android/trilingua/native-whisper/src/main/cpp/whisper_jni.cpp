#include <jni.h>
#include <string>
#include <vector>
#include <android/log.h>

// whisper.cpp public API
#include "whisper.h"

#define LOG_TAG "WhisperJNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO,  LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

extern "C" {

/**
 * Java_com_trilingua_app_nativebridge_Whisper_nativeInit
 * external fun nativeInit(modelPath: String): Long
 */
JNIEXPORT jlong JNICALL
Java_com_trilingua_app_nativebridge_Whisper_nativeInit(
        JNIEnv *env, jclass /*clazz*/, jstring modelPathJ) {

    const char *modelPath = env->GetStringUTFChars(modelPathJ, nullptr);
    if (!modelPath) {
        LOGE("nativeInit: null model path");
        return 0L;
    }

    LOGI("nativeInit: loading model from %s", modelPath);

    struct whisper_context_params cparams = whisper_context_default_params();
    cparams.use_gpu = false; // No GPU on Android arm64 via whisper.cpp CPU path

    struct whisper_context *ctx = whisper_init_from_file_with_params(modelPath, cparams);
    env->ReleaseStringUTFChars(modelPathJ, modelPath);

    if (!ctx) {
        LOGE("nativeInit: whisper_init_from_file_with_params returned null");
        return 0L;
    }

    LOGI("nativeInit: model loaded successfully, ctx=%p", ctx);
    return reinterpret_cast<jlong>(ctx);
}

/**
 * Java_com_trilingua_app_nativebridge_Whisper_nativeFree
 * external fun nativeFree(ctx: Long)
 */
JNIEXPORT void JNICALL
Java_com_trilingua_app_nativebridge_Whisper_nativeFree(
        JNIEnv */*env*/, jobject /*thiz*/, jlong ctxHandle) {

    if (ctxHandle == 0L) return;
    struct whisper_context *ctx = reinterpret_cast<struct whisper_context *>(ctxHandle);
    LOGI("nativeFree: freeing ctx=%p", ctx);
    whisper_free(ctx);
}

/**
 * Java_com_trilingua_app_nativebridge_Whisper_nativeTranscribe
 * external fun nativeTranscribe(ctx: Long, samples: ShortArray, languageTag: String, nThreads: Int): String
 *
 * samples: 16 kHz mono PCM16 as ShortArray
 * Returns UTF-8 transcript string.
 */
JNIEXPORT jstring JNICALL
Java_com_trilingua_app_nativebridge_Whisper_nativeTranscribe(
        JNIEnv *env, jobject /*thiz*/,
        jlong ctxHandle,
        jshortArray samplesJ,
        jstring languageTagJ,
        jint nThreads) {

    if (ctxHandle == 0L) {
        LOGE("nativeTranscribe: null context");
        return env->NewStringUTF("");
    }

    struct whisper_context *ctx = reinterpret_cast<struct whisper_context *>(ctxHandle);

    // Get language tag
    const char *langTag = env->GetStringUTFChars(languageTagJ, nullptr);
    if (!langTag) {
        LOGE("nativeTranscribe: null language tag");
        return env->NewStringUTF("");
    }

    // Convert ShortArray PCM16 -> float32 normalized samples
    jsize numSamples = env->GetArrayLength(samplesJ);
    jshort *rawSamples = env->GetShortArrayElements(samplesJ, nullptr);
    if (!rawSamples) {
        env->ReleaseStringUTFChars(languageTagJ, langTag);
        LOGE("nativeTranscribe: failed to get sample data");
        return env->NewStringUTF("");
    }

    std::vector<float> floatSamples(numSamples);
    for (jsize i = 0; i < numSamples; ++i) {
        floatSamples[i] = static_cast<float>(rawSamples[i]) / 32768.0f;
    }
    env->ReleaseShortArrayElements(samplesJ, rawSamples, JNI_ABORT);

    // Configure whisper params
    struct whisper_full_params params = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    params.print_progress   = false;
    params.print_special    = false;
    params.print_realtime   = false;
    params.print_timestamps = false;
    params.translate        = false;
    params.language         = langTag;
    params.n_threads        = (nThreads > 0) ? nThreads : 4;
    params.single_segment   = false;

    LOGI("nativeTranscribe: running inference, %d samples, lang=%s, threads=%d",
         numSamples, langTag, params.n_threads);

    int ret = whisper_full(ctx, params, floatSamples.data(), static_cast<int>(floatSamples.size()));

    env->ReleaseStringUTFChars(languageTagJ, langTag);

    if (ret != 0) {
        LOGE("nativeTranscribe: whisper_full returned %d", ret);
        return env->NewStringUTF("");
    }

    // Concatenate all segments
    std::string result;
    int nSegments = whisper_full_n_segments(ctx);
    for (int i = 0; i < nSegments; ++i) {
        const char *text = whisper_full_get_segment_text(ctx, i);
        if (text) {
            if (!result.empty() && result.back() != ' ') result += ' ';
            // trim leading space from segment text
            const char *p = text;
            while (*p == ' ') ++p;
            result += p;
        }
    }

    LOGI("nativeTranscribe: got %d segments, result length=%zu", nSegments, result.size());
    return env->NewStringUTF(result.c_str());
}

} // extern "C"
