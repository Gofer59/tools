"""Unicode math glyph → spoken-words table for EN and FR.

Coverage: Greek letters, sub/superscripts, common operators, blackboard letters,
arrows, set theory, comparisons. Sufficient for typical PDF/Unicode math snippets.
For unrecognized glyphs, fall through to LLM normalization.
"""
from __future__ import annotations

# Greek lowercase
_GREEK_L = {
    "α": ("alpha", "alpha"),
    "β": ("beta", "bêta"),
    "γ": ("gamma", "gamma"),
    "δ": ("delta", "delta"),
    "ε": ("epsilon", "epsilon"),
    "ζ": ("zeta", "zêta"),
    "η": ("eta", "êta"),
    "θ": ("theta", "thêta"),
    "ι": ("iota", "iota"),
    "κ": ("kappa", "kappa"),
    "λ": ("lambda", "lambda"),
    "μ": ("mu", "mu"),
    "ν": ("nu", "nu"),
    "ξ": ("xi", "xi"),
    "ο": ("omicron", "omicron"),
    "π": ("pi", "pi"),
    "ρ": ("rho", "rho"),
    "σ": ("sigma", "sigma"),
    "ς": ("final sigma", "sigma final"),
    "τ": ("tau", "tau"),
    "υ": ("upsilon", "upsilon"),
    "φ": ("phi", "phi"),
    "χ": ("chi", "chi"),
    "ψ": ("psi", "psi"),
    "ω": ("omega", "oméga"),
    "ϕ": ("phi", "phi"),
}
_GREEK_U = {
    "Α": ("capital alpha", "alpha majuscule"),
    "Β": ("capital beta", "bêta majuscule"),
    "Γ": ("capital gamma", "gamma majuscule"),
    "Δ": ("capital delta", "delta majuscule"),
    "Ε": ("capital epsilon", "epsilon majuscule"),
    "Ζ": ("capital zeta", "zêta majuscule"),
    "Η": ("capital eta", "êta majuscule"),
    "Θ": ("capital theta", "thêta majuscule"),
    "Ι": ("capital iota", "iota majuscule"),
    "Κ": ("capital kappa", "kappa majuscule"),
    "Λ": ("capital lambda", "lambda majuscule"),
    "Μ": ("capital mu", "mu majuscule"),
    "Ν": ("capital nu", "nu majuscule"),
    "Ξ": ("capital xi", "xi majuscule"),
    "Ο": ("capital omicron", "omicron majuscule"),
    "Π": ("capital pi", "pi majuscule"),
    "Ρ": ("capital rho", "rho majuscule"),
    "Σ": ("capital sigma", "sigma majuscule"),
    "Τ": ("capital tau", "tau majuscule"),
    "Υ": ("capital upsilon", "upsilon majuscule"),
    "Φ": ("capital phi", "phi majuscule"),
    "Χ": ("capital chi", "chi majuscule"),
    "Ψ": ("capital psi", "psi majuscule"),
    "Ω": ("capital omega", "oméga majuscule"),
}

# Operators + relations
_OPS = {
    "∀": ("for all", "pour tout"),
    "∂": ("partial", "partiel"),
    "∃": ("there exists", "il existe"),
    "∄": ("there does not exist", "il n'existe pas"),
    "∅": ("empty set", "ensemble vide"),
    "∇": ("nabla", "nabla"),
    "∈": ("in", "appartient à"),
    "∉": ("not in", "n'appartient pas à"),
    "∋": ("contains", "contient"),
    "∏": ("product", "produit"),
    "∑": ("sum", "somme"),
    "∗": ("times", "fois"),
    "−": ("minus", "moins"),
    "∓": ("minus or plus", "moins ou plus"),
    "±": ("plus or minus", "plus ou moins"),
    "⋅": ("dot", "point"),
    "·": ("dot", "point"),
    "×": ("times", "fois"),
    "÷": ("divided by", "divisé par"),
    "∘": ("composed with", "rond"),
    "√": ("square root of", "racine carrée de"),
    "∛": ("cube root of", "racine cubique de"),
    "∜": ("fourth root of", "racine quatrième de"),
    "∝": ("proportional to", "proportionnel à"),
    "∞": ("infinity", "infini"),
    "∠": ("angle", "angle"),
    "∡": ("measured angle", "angle mesuré"),
    "∢": ("spherical angle", "angle sphérique"),
    "∣": ("divides", "divise"),
    "∤": ("does not divide", "ne divise pas"),
    "∥": ("parallel to", "parallèle à"),
    "∦": ("not parallel to", "non parallèle à"),
    "⊥": ("perpendicular to", "perpendiculaire à"),
    "∧": ("and", "et"),
    "∨": ("or", "ou"),
    "¬": ("not", "non"),
    "⊕": ("direct sum", "somme directe"),
    "⊗": ("tensor product", "produit tensoriel"),
    "∩": ("intersection", "intersection"),
    "∪": ("union", "union"),
    "⊂": ("subset of", "sous-ensemble de"),
    "⊃": ("superset of", "sur-ensemble de"),
    "⊆": ("subset of or equal to", "inclus ou égal"),
    "⊇": ("superset of or equal to", "contient ou égal"),
    "⊄": ("not a subset of", "non inclus"),
    "∫": ("integral of", "intégrale de"),
    "∬": ("double integral of", "intégrale double de"),
    "∭": ("triple integral of", "intégrale triple de"),
    "∮": ("contour integral of", "intégrale de contour de"),
    "∼": ("similar to", "similaire à"),
    "≃": ("asymptotically equal to", "asymptotiquement égal à"),
    "≅": ("congruent to", "congru à"),
    "≈": ("approximately equal to", "approximativement égal à"),
    "≠": ("not equal to", "différent de"),
    "≡": ("equivalent to", "équivalent à"),
    "≤": ("less than or equal to", "inférieur ou égal à"),
    "≥": ("greater than or equal to", "supérieur ou égal à"),
    "≪": ("much less than", "très inférieur à"),
    "≫": ("much greater than", "très supérieur à"),
    "→": ("to", "vers"),
    "←": ("from", "depuis"),
    "↔": ("if and only if", "si et seulement si"),
    "⇒": ("implies", "implique"),
    "⇐": ("is implied by", "est impliqué par"),
    "⇔": ("if and only if", "si et seulement si"),
    "↦": ("maps to", "associe à"),
    "‖": ("norm", "norme"),
    "⟨": ("bra", "crochet ouvrant"),
    "⟩": ("ket", "crochet fermant"),
    "⌊": ("floor of", "partie entière inférieure de"),
    "⌋": ("", ""),
    "⌈": ("ceiling of", "partie entière supérieure de"),
    "⌉": ("", ""),
    "∎": ("end of proof", "fin de la preuve"),
    "ℵ": ("aleph", "aleph"),
    "ℝ": ("R", "R"),
    "ℕ": ("N", "N"),
    "ℤ": ("Z", "Z"),
    "ℚ": ("Q", "Q"),
    "ℂ": ("C", "C"),
    "ℙ": ("P", "P"),
    "𝔽": ("F", "F"),
    "°": ("degrees", "degrés"),
    "%": ("percent", "pour cent"),
}

# Superscripts → "to the X"
_SUPER = {
    "⁰": "0", "¹": "1", "²": "2", "³": "3", "⁴": "4", "⁵": "5",
    "⁶": "6", "⁷": "7", "⁸": "8", "⁹": "9",
    "ⁿ": "n", "ⁱ": "i", "⁺": "+", "⁻": "-", "⁼": "=",
    "⁽": "(", "⁾": ")",
    "ᵃ": "a", "ᵇ": "b", "ᶜ": "c", "ᵈ": "d", "ᵉ": "e",
    "ᵏ": "k", "ᵐ": "m", "ᵒ": "o", "ᵖ": "p", "ᵗ": "t", "ᵘ": "u",
    "ᵛ": "v", "ʷ": "w", "ˣ": "x", "ʸ": "y", "ᶻ": "z",
}

# Subscripts → "sub X"
_SUB = {
    "₀": "0", "₁": "1", "₂": "2", "₃": "3", "₄": "4", "₅": "5",
    "₆": "6", "₇": "7", "₈": "8", "₉": "9",
    "ₙ": "n", "ᵢ": "i", "ⱼ": "j", "ₖ": "k", "ₐ": "a", "ₑ": "e",
    "ₒ": "o", "ᵤ": "u", "ₓ": "x", "ₕ": "h", "ₗ": "l", "ₘ": "m",
    "ₚ": "p", "ₛ": "s", "ₜ": "t", "₊": "+", "₋": "-", "₌": "=",
    "₍": "(", "₎": ")",
}


def _word_super(c: str, lang: str) -> str:
    sym = _SUPER[c]
    if lang == "fr":
        if sym == "2":
            return " au carré"
        if sym == "3":
            return " au cube"
        return f" puissance {sym}"
    if sym == "2":
        return " squared"
    if sym == "3":
        return " cubed"
    return f" to the {sym}"


def _word_sub(c: str, lang: str) -> str:
    sym = _SUB[c]
    if lang == "fr":
        return f" indice {sym}"
    return f" sub {sym}"


def _pick(pair: tuple[str, str], lang: str) -> str:
    return pair[1] if lang == "fr" else pair[0]


def translate(text: str, lang: str) -> str:
    """Replace each known glyph with its localized spoken form. Unknown glyphs
    are kept verbatim so the caller can detect partial coverage."""
    out: list[str] = []
    for ch in text:
        if ch in _SUPER:
            out.append(_word_super(ch, lang))
        elif ch in _SUB:
            out.append(_word_sub(ch, lang))
        elif ch in _GREEK_L:
            out.append(" " + _pick(_GREEK_L[ch], lang) + " ")
        elif ch in _GREEK_U:
            out.append(" " + _pick(_GREEK_U[ch], lang) + " ")
        elif ch in _OPS:
            w = _pick(_OPS[ch], lang)
            out.append(" " + w + " " if w else "")
        elif ch == "+":
            out.append(" plus " if lang == "en" else " plus ")
        elif ch == "=":
            out.append(" equals " if lang == "en" else " égale ")
        elif ch == "<":
            out.append(" less than " if lang == "en" else " inférieur à ")
        elif ch == ">":
            out.append(" greater than " if lang == "en" else " supérieur à ")
        else:
            out.append(ch)
    s = "".join(out)
    # collapse repeated spaces
    return " ".join(s.split())
