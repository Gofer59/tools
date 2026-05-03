from math_speak.classify import classify


def test_unicode_only():
    assert classify("∫₀¹ x² dx") == "unicode"


def test_latex():
    assert classify(r"\frac{\partial L}{\partial \theta}") == "latex"


def test_latex_dollars():
    assert classify(r"$\sum_{i=1}^n \sqrt{x_i}$") == "latex"


def test_ascii():
    assert classify(r"sum_{i=1}^n sqrt(x_i)") == "ascii"


def test_ascii_simple_power():
    assert classify("x^2 + y^2 = r^2") == "ascii"


def test_pdf_glyphs_only():
    # All-symbolic PDF copy with only single-letter identifiers stays in unicode path.
    assert classify("f : ℝⁿ → ℝ, f(x) = ‖x‖²") == "unicode"


def test_unicode_with_dx_kept_short():
    # "dx" is only 2 chars → stays unicode
    assert classify("∫ x dx") == "unicode"


def test_pdf_with_prose():
    # Prose interleaved with math glyphs → LLM path
    assert classify("Let f denote the function ℝⁿ → ℝ defined by ‖x‖²") == "mixed"


def test_empty():
    assert classify("") == "mixed"
