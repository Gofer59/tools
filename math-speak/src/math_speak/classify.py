from __future__ import annotations

import re

LATEX_RE = re.compile(r"\\[a-zA-Z]+|\\frac|\\sum|\\int|\\sqrt|\\begin\{|\$.+?\$")
MATHML_RE = re.compile(r"<math\b|</math>|<mfrac|<msup|<msub")
ASCII_MATH_RE = re.compile(r"\^|_\{|sqrt\(|int_|sum_|\\?lim_")
UNICODE_MATH_RE = re.compile(
    r"["
    r"⁰-₟"   # super/subscripts
    r"Ͱ-Ͽ"   # Greek
    r"∀-⋿"   # math operators
    r"⨀-⫿"   # supplemental math operators
    r"←-⇿"   # arrows
    r"℀-⅏"   # letterlike (ℝ ℕ ℤ ℂ ℚ)
    r"⁺-ⁿ"
    r"₀-₎"
    r"]"
)


def classify(text: str) -> str:
    """Return one of: latex, mathml, ascii, unicode, mixed."""
    if not text:
        return "mixed"
    if MATHML_RE.search(text):
        return "mathml"
    if LATEX_RE.search(text):
        return "latex"
    has_unicode = bool(UNICODE_MATH_RE.search(text))
    has_ascii_math = bool(ASCII_MATH_RE.search(text))
    if has_unicode and not has_ascii_math:
        # If text contains any 3+ letter prose word, route to mixed (LLM).
        # Otherwise the unicode glyph table handles it.
        if re.search(r"[A-Za-zÀ-ÖØ-öø-ÿ]{3,}", text):
            return "mixed"
        return "unicode"
    if has_ascii_math:
        return "ascii"
    if has_unicode:
        return "mixed"
    return "mixed"
