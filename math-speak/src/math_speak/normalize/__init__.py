from __future__ import annotations

import logging
from pathlib import Path

from ..classify import classify
from ..config import STATE_DIR, Config
from . import fr_polish, latex_pre, llm_rewrite, sre_client, unicode_table

log = logging.getLogger("math_speak.normalize")


def _log_unparsed(text: str, lang: str, kind: str) -> None:
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    path: Path = STATE_DIR / "unparsed.log"
    with path.open("a", encoding="utf-8") as f:
        f.write(f"[{kind}|{lang}] {text!r}\n")


def _finalize(spoken: str, lang: str) -> str:
    if lang == "fr":
        return fr_polish.polish(spoken)
    return spoken


def normalize(text: str, cfg: Config) -> tuple[str, str]:
    """Return (spoken_text, engine_hint).

    engine_hint is "piper" if the text was successfully normalized,
    "espeak" if we should fall through to espeak-ng raw read.
    """
    if not text.strip():
        return "", "piper"

    lang = cfg.language
    if cfg.raw_mode:
        return text, "espeak"

    kind = classify(text)
    log.info("classified as %s (lang=%s, len=%d)", kind, lang, len(text))

    if kind == "latex":
        pre = latex_pre.preprocess(text)
        spoken = sre_client.to_speech(pre, "latex", lang, cfg.sre_domain)
        if spoken:
            return _finalize(spoken, lang), "piper"
        _log_unparsed(text, lang, "latex-failed")
        # fall through

    if kind == "mathml":
        spoken = sre_client.to_speech(text, "mathml", lang, cfg.sre_domain)
        if spoken:
            return _finalize(spoken, lang), "piper"
        _log_unparsed(text, lang, "mathml-failed")

    if kind == "ascii":
        spoken = sre_client.to_speech(text, "asciimath", lang, cfg.sre_domain)
        if spoken:
            return _finalize(spoken, lang), "piper"
        _log_unparsed(text, lang, "ascii-failed")

    if kind == "unicode":
        spoken = unicode_table.translate(text, lang)
        if spoken and spoken.strip():
            return spoken, "piper"

    # mixed or all-failed → try LLM, else espeak raw
    spoken = llm_rewrite.rewrite(text, lang, cfg)
    if spoken:
        return spoken, "piper"

    if cfg.espeak_fallback:
        return text, "espeak"
    return text, "piper"
