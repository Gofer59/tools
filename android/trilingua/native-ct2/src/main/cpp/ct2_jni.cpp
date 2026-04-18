#include <jni.h>
#include <string>
#include <vector>
#include <memory>
#include <android/log.h>

// CTranslate2 public API
#include "ctranslate2/translator.h"
#include "ctranslate2/devices.h"
#include "ctranslate2/types.h"

// SentencePiece
#include "sentencepiece_processor.h"

#define LOG_TAG "CT2JNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO,  LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// A handle struct pairing CT2 Translator + cached SPM processors
struct CT2Handle {
    std::unique_ptr<ctranslate2::Translator> translator;
    sentencepiece::SentencePieceProcessor srcSpm;
    sentencepiece::SentencePieceProcessor tgtSpm;
    int numThreads;
};

extern "C" {

/**
 * Java_com_trilingua_app_nativebridge_Ct2Translator_nativeOpen
 * external fun nativeOpen(modelDir: String, srcSpmPath: String, tgtSpmPath: String, numThreads: Int): Long
 */
JNIEXPORT jlong JNICALL
Java_com_trilingua_app_nativebridge_Ct2Translator_nativeOpen(
        JNIEnv *env, jclass /*clazz*/,
        jstring modelDirJ, jstring srcSpmPathJ, jstring tgtSpmPathJ, jint numThreads) {

    const char *modelDir    = env->GetStringUTFChars(modelDirJ,    nullptr);
    const char *srcSpmPath  = env->GetStringUTFChars(srcSpmPathJ,  nullptr);
    const char *tgtSpmPath  = env->GetStringUTFChars(tgtSpmPathJ,  nullptr);

    if (!modelDir || !srcSpmPath || !tgtSpmPath) {
        LOGE("nativeOpen: null argument");
        if (modelDir)   env->ReleaseStringUTFChars(modelDirJ,   modelDir);
        if (srcSpmPath) env->ReleaseStringUTFChars(srcSpmPathJ, srcSpmPath);
        if (tgtSpmPath) env->ReleaseStringUTFChars(tgtSpmPathJ, tgtSpmPath);
        return 0L;
    }

    LOGI("nativeOpen: loading CT2 model from %s, threads=%d", modelDir, numThreads);

    try {
        ctranslate2::ReplicaPoolConfig poolConfig;
        poolConfig.num_threads_per_replica = (numThreads > 0) ? (size_t)numThreads : 2;
        poolConfig.max_queued_batches = 0;
        poolConfig.cpu_core_offset = -1;

        auto handle = new CT2Handle();
        handle->numThreads = (numThreads > 0) ? numThreads : 2;
        handle->translator = std::make_unique<ctranslate2::Translator>(
            std::string(modelDir),
            ctranslate2::Device::CPU,
            ctranslate2::ComputeType::INT8,
            /*device_indices=*/std::vector<int>{0},
            /*tensor_parallel=*/false,
            poolConfig
        );

        // Load source SPM once and cache it in the handle
        auto srcStatus = handle->srcSpm.Load(std::string(srcSpmPath));
        if (!srcStatus.ok()) {
            LOGE("nativeOpen: failed to load src spm: %s", srcStatus.ToString().c_str());
            delete handle;
            env->ReleaseStringUTFChars(modelDirJ,   modelDir);
            env->ReleaseStringUTFChars(srcSpmPathJ, srcSpmPath);
            env->ReleaseStringUTFChars(tgtSpmPathJ, tgtSpmPath);
            return 0L;
        }

        // Load target SPM once and cache it in the handle
        auto tgtStatus = handle->tgtSpm.Load(std::string(tgtSpmPath));
        if (!tgtStatus.ok()) {
            LOGE("nativeOpen: failed to load tgt spm: %s", tgtStatus.ToString().c_str());
            delete handle;
            env->ReleaseStringUTFChars(modelDirJ,   modelDir);
            env->ReleaseStringUTFChars(srcSpmPathJ, srcSpmPath);
            env->ReleaseStringUTFChars(tgtSpmPathJ, tgtSpmPath);
            return 0L;
        }

        env->ReleaseStringUTFChars(modelDirJ,   modelDir);
        env->ReleaseStringUTFChars(srcSpmPathJ, srcSpmPath);
        env->ReleaseStringUTFChars(tgtSpmPathJ, tgtSpmPath);
        LOGI("nativeOpen: model + SPMs loaded, handle=%p", handle);
        return reinterpret_cast<jlong>(handle);

    } catch (const std::exception &e) {
        LOGE("nativeOpen: exception: %s", e.what());
        env->ReleaseStringUTFChars(modelDirJ,   modelDir);
        env->ReleaseStringUTFChars(srcSpmPathJ, srcSpmPath);
        env->ReleaseStringUTFChars(tgtSpmPathJ, tgtSpmPath);
        return 0L;
    }
}

/**
 * Java_com_trilingua_app_nativebridge_Ct2Translator_nativeClose
 * external fun nativeClose(handle: Long)
 */
JNIEXPORT void JNICALL
Java_com_trilingua_app_nativebridge_Ct2Translator_nativeClose(
        JNIEnv */*env*/, jobject /*thiz*/, jlong handleJ) {
    if (handleJ == 0L) return;
    CT2Handle *handle = reinterpret_cast<CT2Handle *>(handleJ);
    LOGI("nativeClose: freeing handle=%p", handle);
    delete handle;
}

/**
 * Java_com_trilingua_app_nativebridge_Ct2Translator_nativeTranslate
 * external fun nativeTranslate(handle: Long, text: String, beamSize: Int, maxDecodingLength: Int): String
 *
 * SPM is now cached in the handle; no path args needed per call.
 */
JNIEXPORT jstring JNICALL
Java_com_trilingua_app_nativebridge_Ct2Translator_nativeTranslate(
        JNIEnv *env, jobject /*thiz*/,
        jlong handleJ,
        jstring textJ,
        jint beamSize, jint maxDecodingLength) {

    if (handleJ == 0L) {
        LOGE("nativeTranslate: null handle");
        return env->NewStringUTF("");
    }

    CT2Handle *handle = reinterpret_cast<CT2Handle *>(handleJ);

    const char *text = env->GetStringUTFChars(textJ, nullptr);
    if (!text) {
        LOGE("nativeTranslate: null text");
        return env->NewStringUTF("");
    }

    try {
        // Encode source text using cached SPM
        std::vector<std::string> srcTokens;
        handle->srcSpm.Encode(std::string(text), &srcTokens);

        // OPUS-MT / Marian expects EOS on source; without it the decoder won't emit </s>
        // and loops until max_decoding_length, producing repetitive output.
        srcTokens.push_back("</s>");

        LOGI("nativeTranslate: src tokens count=%zu, text='%s'", srcTokens.size(), text);

        // Set up translation options
        ctranslate2::TranslationOptions opts;
        opts.beam_size = (beamSize > 0) ? beamSize : 4;
        opts.max_decoding_length = (maxDecodingLength > 0) ? maxDecodingLength : 256;
        opts.num_hypotheses = 1;
        opts.repetition_penalty = 1.1f;        // discourage exact-token repetition
        opts.no_repeat_ngram_size = 3;         // block exact trigram repeats
        opts.length_penalty = 1.0f;
        opts.return_scores = false;

        // Run translation
        std::vector<std::vector<std::string>> batch = {srcTokens};
        auto results = handle->translator->translate_batch(batch, opts);

        if (results.empty() || results[0].hypotheses.empty()) {
            LOGE("nativeTranslate: empty result");
            env->ReleaseStringUTFChars(textJ, text);
            return env->NewStringUTF("");
        }

        // Decode target tokens using cached SPM
        const auto &tgtTokens = results[0].hypotheses[0];
        std::string decoded;
        handle->tgtSpm.Decode(tgtTokens, &decoded);

        LOGI("nativeTranslate: decoded='%s'", decoded.c_str());

        env->ReleaseStringUTFChars(textJ, text);

        return env->NewStringUTF(decoded.c_str());

    } catch (const std::exception &e) {
        LOGE("nativeTranslate: exception: %s", e.what());
        env->ReleaseStringUTFChars(textJ, text);
        return env->NewStringUTF("");
    }
}

} // extern "C"
