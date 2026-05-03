"""Post-process SRE French output to fix common rough edges where the
ClearSpeak FR rules leak English fragments or wrong articles.

Applied only to FR output, after SRE returns its string.
"""
from __future__ import annotations

import re

# Order matters: longer phrases first so they don't get clobbered by shorter ones.
_REPLACEMENTS: list[tuple[re.Pattern[str], str]] = [
    # Wrong articles before vowel-initial nouns
    (re.compile(r"\ble intégrale\b", re.IGNORECASE), "l'intégrale"),
    (re.compile(r"\ble somme\b", re.IGNORECASE), "la somme"),
    (re.compile(r"\ble sommation\b", re.IGNORECASE), "la somme"),
    (re.compile(r"\ble produit\b"), "le produit"),
    (re.compile(r"\ble racine\b", re.IGNORECASE), "la racine"),
    (re.compile(r"\bla racine carrée de la\b", re.IGNORECASE), "la racine carrée de"),
    (re.compile(r"\ble fraction\b", re.IGNORECASE), "la fraction"),
    (re.compile(r"\ble dérivée\b", re.IGNORECASE), "la dérivée"),
    (re.compile(r"\ble limite\b", re.IGNORECASE), "la limite"),
    (re.compile(r"\bde le\b"), "du"),
    (re.compile(r"\bde les\b"), "des"),
    # English leakage
    (re.compile(r"\bsub\b"), "indice"),
    (re.compile(r"\bsup\b"), "exposant"),
    (re.compile(r"\bsuper\b"), "exposant"),
    # Operator word fixes
    (re.compile(r"\bsommation\b"), "somme"),
]


def polish(text: str) -> str:
    if not text:
        return text
    out = text
    for pat, repl in _REPLACEMENTS:
        out = pat.sub(repl, out)
    # collapse repeated whitespace
    return " ".join(out.split())
