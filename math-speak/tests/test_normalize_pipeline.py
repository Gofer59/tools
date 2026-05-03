"""Pipeline-level tests that don't require SRE / LLM / piper to be running."""

from math_speak.classify import classify
from math_speak.config import Config
from math_speak.normalize import normalize


def _cfg(**overrides):
    base = dict(language="en", espeak_fallback=True, llm_timeout_s=0.05)
    base.update(overrides)
    return Config(**base)


def test_unicode_path_en():
    spoken, engine = normalize("α + β", _cfg(language="en"))
    # SRE is offline → unicode_table should still produce English text
    assert "alpha" in spoken
    assert "beta" in spoken
    assert engine in ("piper", "espeak")


def test_unicode_path_fr():
    spoken, _ = normalize("α + β", _cfg(language="fr"))
    assert "alpha" in spoken
    assert "bêta" in spoken


def test_raw_mode():
    text = "anything here"
    spoken, engine = normalize(text, _cfg(raw_mode=True))
    assert spoken == text
    assert engine == "espeak"


def test_empty():
    spoken, engine = normalize("", _cfg())
    assert spoken == ""


def test_classify_dispatch_consistency():
    cases = [
        ("∫₀¹ x² dx", "unicode"),
        (r"\frac{a}{b}", "latex"),
        (r"sum_{i=1}^n sqrt(x_i)", "ascii"),
        ("f : ℝⁿ → ℝ, f(x) = norm", "mixed"),
    ]
    for text, expected in cases:
        assert classify(text) == expected, f"{text!r} → expected {expected}"
