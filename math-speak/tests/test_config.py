from pathlib import Path

import pytest


@pytest.fixture
def tmp_xdg(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "config"))
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / "data"))
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path / "state"))
    # Force re-import to pick up env vars
    import importlib

    import math_speak.config as cfgmod
    importlib.reload(cfgmod)
    return tmp_path


def test_default_load_creates_file(tmp_xdg):
    import math_speak.config as cfgmod
    cfg = cfgmod.load()
    assert cfg.language == "en"
    assert cfg.hotkey == "<ctrl>+<alt>+m"
    assert Path(cfgmod.CONFIG_PATH).exists()


def test_set_language(tmp_xdg):
    import math_speak.config as cfgmod
    cfgmod.set_language("fr")
    assert cfgmod.load().language == "fr"
    cfgmod.set_language("en")
    assert cfgmod.load().language == "en"


def test_voice_for_language(tmp_xdg):
    import math_speak.config as cfgmod
    cfg = cfgmod.Config(language="fr", piper_voice_fr="fr_FR-siwis-medium")
    assert cfg.voice_for_language() == "fr_FR-siwis-medium"
    cfg.language = "en"
    assert cfg.voice_for_language().startswith("en_")
