from __future__ import annotations

import hashlib
import logging
from collections import OrderedDict

import httpx

from ..config import Config

log = logging.getLogger("math_speak.llm")
_cache: OrderedDict[str, str] = OrderedDict()
_CACHE_MAX = 500

_PROMPTS = {
    "en": (
        "Rewrite this mathematical expression as fluent spoken English. "
        "Expand all symbols, operators, fractions, sub/superscripts, Greek letters, "
        "and integral/sum/limit notation into ordinary words. "
        "Output only the spoken sentence, no commentary, no quotes.\n\nInput:\n{text}"
    ),
    "fr": (
        "Réécris cette expression mathématique en français parlé courant. "
        "Développe tous les symboles, opérateurs, fractions, indices, exposants, lettres grecques, "
        "et les notations d'intégrale, de somme et de limite en mots ordinaires. "
        "Donne uniquement la phrase parlée, sans commentaire ni guillemets.\n\nEntrée :\n{text}"
    ),
}


def _cache_get(key: str) -> str | None:
    if key in _cache:
        _cache.move_to_end(key)
        return _cache[key]
    return None


def _cache_put(key: str, val: str) -> None:
    _cache[key] = val
    _cache.move_to_end(key)
    while len(_cache) > _CACHE_MAX:
        _cache.popitem(last=False)


def warmup(cfg: Config) -> None:
    try:
        httpx.post(
            f"{cfg.llm_endpoint}/api/generate",
            json={"model": cfg.llm_model, "prompt": "ping", "stream": False},
            timeout=2.0,
        )
    except Exception as e:
        log.info("LLM warmup skipped: %s", e)


def rewrite(text: str, lang: str, cfg: Config) -> str:
    if not text.strip():
        return ""
    key = hashlib.sha256(f"{lang}|{text}".encode()).hexdigest()
    cached = _cache_get(key)
    if cached is not None:
        return cached
    prompt = _PROMPTS.get(lang, _PROMPTS["en"]).format(text=text)
    try:
        r = httpx.post(
            f"{cfg.llm_endpoint}/api/generate",
            json={
                "model": cfg.llm_model,
                "prompt": prompt,
                "stream": False,
                "options": {"temperature": 0.1, "num_predict": 256},
            },
            timeout=cfg.llm_timeout_s,
        )
        r.raise_for_status()
        data = r.json()
    except Exception as e:
        log.warning("LLM rewrite failed: %s", e)
        return ""
    spoken = (data.get("response") or "").strip().strip('"').strip("'")
    if spoken:
        _cache_put(key, spoken)
    return spoken
