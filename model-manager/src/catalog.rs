use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelKind {
    Whisper,
    Piper,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelEntry {
    pub id: &'static str,
    pub kind: ModelKind,
    pub display_name: &'static str,
    pub language: &'static str,
    pub size_bytes: u64,
    pub license: &'static str,
    pub urls: &'static [&'static str],
    pub sha256: Option<&'static str>,
    pub multilingual: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperModel {
    pub entry: ModelEntry,
}

#[derive(Debug, Clone, Serialize)]
pub struct PiperVoice {
    pub entry: ModelEntry,
}

pub static WHISPER_MODELS: &[ModelEntry] = &[
    ModelEntry {
        id: "tiny",
        kind: ModelKind::Whisper,
        display_name: "Whisper Tiny",
        language: "multi",
        size_bytes: 75_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-whisper-tiny/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-whisper-tiny/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-whisper-tiny/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-whisper-tiny/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: true,
    },
    ModelEntry {
        id: "base",
        kind: ModelKind::Whisper,
        display_name: "Whisper Base",
        language: "multi",
        size_bytes: 145_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-whisper-base/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-whisper-base/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-whisper-base/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-whisper-base/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: true,
    },
    ModelEntry {
        id: "distil-small.en",
        kind: ModelKind::Whisper,
        display_name: "Whisper Distil Small (en)",
        language: "en",
        size_bytes: 332_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-distil-whisper-small.en/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-distil-whisper-small.en/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-distil-whisper-small.en/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-distil-whisper-small.en/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "small",
        kind: ModelKind::Whisper,
        display_name: "Whisper Small",
        language: "multi",
        size_bytes: 466_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-whisper-small/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-whisper-small/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-whisper-small/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-whisper-small/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: true,
    },
    ModelEntry {
        id: "distil-large-v3",
        kind: ModelKind::Whisper,
        display_name: "Whisper Distil Large v3",
        language: "en",
        size_bytes: 1_510_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-distil-whisper-large-v3/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-distil-whisper-large-v3/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-distil-whisper-large-v3/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-distil-whisper-large-v3/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "medium.en",
        kind: ModelKind::Whisper,
        display_name: "Whisper Medium (en)",
        language: "en",
        size_bytes: 1_528_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-whisper-medium.en/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-whisper-medium.en/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-whisper-medium.en/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-whisper-medium.en/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "medium",
        kind: ModelKind::Whisper,
        display_name: "Whisper Medium",
        language: "multi",
        size_bytes: 1_528_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-whisper-medium/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-whisper-medium/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-whisper-medium/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-whisper-medium/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: true,
    },
    ModelEntry {
        id: "large-v2",
        kind: ModelKind::Whisper,
        display_name: "Whisper Large v2",
        language: "multi",
        size_bytes: 2_887_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-whisper-large-v2/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-whisper-large-v2/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-whisper-large-v2/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-whisper-large-v2/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: true,
    },
    ModelEntry {
        id: "large-v3",
        kind: ModelKind::Whisper,
        display_name: "Whisper Large v3",
        language: "multi",
        size_bytes: 2_887_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/Systran/faster-whisper-large-v3/resolve/main/model.bin",
            "https://huggingface.co/Systran/faster-whisper-large-v3/resolve/main/config.json",
            "https://huggingface.co/Systran/faster-whisper-large-v3/resolve/main/tokenizer.json",
            "https://huggingface.co/Systran/faster-whisper-large-v3/resolve/main/vocabulary.txt",
        ],
        sha256: None,
        multilingual: true,
    },
];

pub static PIPER_VOICES: &[ModelEntry] = &[
    // en-US
    ModelEntry {
        id: "en_US-lessac-medium",
        kind: ModelKind::Piper,
        display_name: "Lessac (en-US)",
        language: "en-US",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "en_US-ryan-medium",
        kind: ModelKind::Piper,
        display_name: "Ryan (en-US)",
        language: "en-US",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium/en_US-ryan-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium/en_US-ryan-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "en_US-amy-medium",
        kind: ModelKind::Piper,
        display_name: "Amy (en-US)",
        language: "en-US",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/amy/medium/en_US-amy-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/amy/medium/en_US-amy-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "en_US-hfc_female-medium",
        kind: ModelKind::Piper,
        display_name: "HFC Female (en-US)",
        language: "en-US",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/hfc_female/medium/en_US-hfc_female-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/hfc_female/medium/en_US-hfc_female-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // en-GB
    ModelEntry {
        id: "en_GB-alan-medium",
        kind: ModelKind::Piper,
        display_name: "Alan (en-GB)",
        language: "en-GB",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/alan/medium/en_GB-alan-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/alan/medium/en_GB-alan-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "en_GB-jenny_dioco-medium",
        kind: ModelKind::Piper,
        display_name: "Jenny Dioco (en-GB)",
        language: "en-GB",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/jenny_dioco/medium/en_GB-jenny_dioco-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/jenny_dioco/medium/en_GB-jenny_dioco-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // fr-FR
    ModelEntry {
        id: "fr_FR-siwis-medium",
        kind: ModelKind::Piper,
        display_name: "Siwis (fr-FR)",
        language: "fr-FR",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium/fr_FR-siwis-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium/fr_FR-siwis-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "fr_FR-upmc-medium",
        kind: ModelKind::Piper,
        display_name: "UPMC (fr-FR)",
        language: "fr-FR",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/upmc/medium/fr_FR-upmc-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/upmc/medium/fr_FR-upmc-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // de-DE
    ModelEntry {
        id: "de_DE-thorsten-medium",
        kind: ModelKind::Piper,
        display_name: "Thorsten (de-DE)",
        language: "de-DE",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/de/de_DE/thorsten/medium/de_DE-thorsten-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/de/de_DE/thorsten/medium/de_DE-thorsten-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "de_DE-ramona-medium",
        kind: ModelKind::Piper,
        display_name: "Ramona (de-DE)",
        language: "de-DE",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/de/de_DE/ramona_deininger/medium/de_DE-ramona_deininger-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/de/de_DE/ramona_deininger/medium/de_DE-ramona_deininger-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // es-ES
    ModelEntry {
        id: "es_ES-mls_10246-medium",
        kind: ModelKind::Piper,
        display_name: "MLS 10246 (es-ES)",
        language: "es-ES",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/es/es_ES/mls_10246/medium/es_ES-mls_10246-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/es/es_ES/mls_10246/medium/es_ES-mls_10246-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // es-MX
    ModelEntry {
        id: "es_MX-claude-medium",
        kind: ModelKind::Piper,
        display_name: "Claude (es-MX)",
        language: "es-MX",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/es/es_MX/claude/medium/es_MX-claude-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/es/es_MX/claude/medium/es_MX-claude-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // it-IT
    ModelEntry {
        id: "it_IT-paola-medium",
        kind: ModelKind::Piper,
        display_name: "Paola (it-IT)",
        language: "it-IT",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/it/it_IT/paola/medium/it_IT-paola-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/it/it_IT/paola/medium/it_IT-paola-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "it_IT-riccardo-x_low",
        kind: ModelKind::Piper,
        display_name: "Riccardo (it-IT, x_low)",
        language: "it-IT",
        size_bytes: 22_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/it/it_IT/riccardo/x_low/it_IT-riccardo-x_low.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/it/it_IT/riccardo/x_low/it_IT-riccardo-x_low.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // zh-CN
    ModelEntry {
        id: "zh_CN-huayan-medium",
        kind: ModelKind::Piper,
        display_name: "Huayan (zh-CN)",
        language: "zh-CN",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    ModelEntry {
        id: "zh_CN-huayan-x_low",
        kind: ModelKind::Piper,
        display_name: "Huayan (zh-CN, x_low)",
        language: "zh-CN",
        size_bytes: 22_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/zh/zh_CN/huayan/x_low/zh_CN-huayan-x_low.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/zh/zh_CN/huayan/x_low/zh_CN-huayan-x_low.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // pt-BR
    ModelEntry {
        id: "pt_BR-faber-medium",
        kind: ModelKind::Piper,
        display_name: "Faber (pt-BR)",
        language: "pt-BR",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // ru-RU
    ModelEntry {
        id: "ru_RU-irina-medium",
        kind: ModelKind::Piper,
        display_name: "Irina (ru-RU)",
        language: "ru-RU",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/ru/ru_RU/irina/medium/ru_RU-irina-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/ru/ru_RU/irina/medium/ru_RU-irina-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // nl-NL
    ModelEntry {
        id: "nl_NL-mls_5809-medium",
        kind: ModelKind::Piper,
        display_name: "MLS 5809 (nl-NL)",
        language: "nl-NL",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/nl/nl_NL/mls_5809/medium/nl_NL-mls_5809-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/nl/nl_NL/mls_5809/medium/nl_NL-mls_5809-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // pl-PL
    ModelEntry {
        id: "pl_PL-gosia-medium",
        kind: ModelKind::Piper,
        display_name: "Gosia (pl-PL)",
        language: "pl-PL",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pl/pl_PL/gosia/medium/pl_PL-gosia-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/pl/pl_PL/gosia/medium/pl_PL-gosia-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // sv-SE
    ModelEntry {
        id: "sv_SE-nst-medium",
        kind: ModelKind::Piper,
        display_name: "NST (sv-SE)",
        language: "sv-SE",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/sv/sv_SE/nst/medium/sv_SE-nst-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/sv/sv_SE/nst/medium/sv_SE-nst-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // tr-TR
    ModelEntry {
        id: "tr_TR-dfki-medium",
        kind: ModelKind::Piper,
        display_name: "DFKI (tr-TR)",
        language: "tr-TR",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/tr/tr_TR/dfki/medium/tr_TR-dfki-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/tr/tr_TR/dfki/medium/tr_TR-dfki-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
    // uk-UA
    ModelEntry {
        id: "uk_UA-ukrainian_tts-medium",
        kind: ModelKind::Piper,
        display_name: "Ukrainian TTS (uk-UA)",
        language: "uk-UA",
        size_bytes: 63_000_000,
        license: "MIT",
        urls: &[
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/uk/uk_UA/ukrainian_tts/medium/uk_UA-ukrainian_tts-medium.onnx",
            "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/uk/uk_UA/ukrainian_tts/medium/uk_UA-ukrainian_tts-medium.onnx.json",
        ],
        sha256: None,
        multilingual: false,
    },
];

pub fn whisper_by_id(id: &str) -> Option<&'static ModelEntry> {
    WHISPER_MODELS.iter().find(|m| m.id == id)
}

pub fn piper_by_id(id: &str) -> Option<&'static ModelEntry> {
    PIPER_VOICES.iter().find(|m| m.id == id)
}

pub fn entries_for(kind: ModelKind) -> &'static [ModelEntry] {
    match kind {
        ModelKind::Whisper => WHISPER_MODELS,
        ModelKind::Piper => PIPER_VOICES,
    }
}
