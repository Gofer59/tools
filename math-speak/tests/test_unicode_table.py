from math_speak.normalize.unicode_table import translate


def test_integral_en():
    out = translate("∫₀¹ x² dx", "en")
    assert "integral of" in out
    assert "sub 0" in out
    assert "to the 1" in out
    assert "squared" in out


def test_integral_fr():
    out = translate("∫₀¹ x² dx", "fr")
    assert "intégrale de" in out
    assert "indice 0" in out
    assert "puissance 1" in out
    assert "carré" in out


def test_greek_en():
    out = translate("α + β ≤ γ", "en")
    assert "alpha" in out and "beta" in out and "gamma" in out
    assert "less than or equal to" in out


def test_greek_fr():
    out = translate("α + β ≤ γ", "fr")
    assert "alpha" in out and "bêta" in out and "gamma" in out
    assert "inférieur ou égal à" in out


def test_nabla_en():
    out = translate("∇·E = ρ/ε₀", "en")
    assert "nabla" in out
    assert "dot" in out
    assert "rho" in out
    assert "epsilon" in out
    assert "sub 0" in out


def test_nabla_fr():
    out = translate("∇·E = ρ/ε₀", "fr")
    assert "nabla" in out
    assert "rho" in out and "epsilon" in out
    assert "indice 0" in out


def test_norm_squared_en():
    out = translate("‖x‖²", "en")
    assert "norm" in out
    assert "squared" in out


def test_norm_squared_fr():
    out = translate("‖x‖²", "fr")
    assert "norme" in out
    assert "carré" in out
