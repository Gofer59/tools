from __future__ import annotations

from pylatexenc.latex2text import LatexNodes2Text

_converter = LatexNodes2Text(math_mode="verbatim", strict_latex_spaces=False)


def preprocess(text: str) -> str:
    """Strip text-mode LaTeX (\\textit, \\emph, accents, etc.) but leave math markup
    (\\frac, \\sum, \\int, etc.) intact for SRE."""
    text = text.replace("$$", "$").strip()
    return _converter.latex_to_text(text).strip()
