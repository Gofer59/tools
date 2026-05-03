from __future__ import annotations

import os
import tomllib
from dataclasses import asdict, dataclass, field
from pathlib import Path

import tomli_w

CONFIG_DIR = Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "math-speak"
CONFIG_PATH = CONFIG_DIR / "config.toml"
DATA_DIR = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share") / "math-speak"
STATE_DIR = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state") / "math-speak"


@dataclass
class Config:
    hotkey: str = "<ctrl>+<alt>+m"
    language: str = "en"
    # amy-medium is a natural-sounding female US voice; siwis is the standard
    # female FR voice from the SIWIS corpus.
    piper_voice_en: str = "en_US-amy-medium"
    piper_voice_fr: str = "fr_FR-siwis-medium"
    espeak_fallback: bool = True
    llm_endpoint: str = "http://127.0.0.1:11434"
    llm_model: str = "gemma3n:e2b"
    llm_timeout_s: float = 3.0
    model_dir: str = field(default_factory=lambda: str(DATA_DIR / "models"))
    speed: float = 1.0
    raw_mode: bool = False  # skip normalizer; speak input as-is via espeak-ng
    # SRE phrasing style: "clearspeak" (natural prose) or "mathspeak" (formal).
    sre_domain: str = "clearspeak"

    def voice_for_language(self) -> str:
        return self.piper_voice_fr if self.language == "fr" else self.piper_voice_en

    def expanded_model_dir(self) -> Path:
        return Path(os.path.expanduser(self.model_dir))


def load() -> Config:
    if not CONFIG_PATH.exists():
        cfg = Config()
        save(cfg)
        return cfg
    with CONFIG_PATH.open("rb") as f:
        data = tomllib.load(f)
    cfg = Config()
    for k, v in data.items():
        if hasattr(cfg, k):
            setattr(cfg, k, v)
    return cfg


def save(cfg: Config) -> None:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    with CONFIG_PATH.open("wb") as f:
        tomli_w.dump(asdict(cfg), f)


def set_language(lang: str) -> Config:
    cfg = load()
    cfg.language = "fr" if lang == "fr" else "en"
    save(cfg)
    return cfg
