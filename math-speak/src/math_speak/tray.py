from __future__ import annotations

import logging
import threading
from collections.abc import Callable

log = logging.getLogger("math_speak.tray")


def _icon(text: str, bg: tuple[int, int, int]):
    from PIL import Image, ImageDraw

    img = Image.new("RGB", (64, 64), bg)
    d = ImageDraw.Draw(img)
    d.ellipse((4, 4, 60, 60), fill=bg, outline=(255, 255, 255), width=2)
    d.text((16, 18), text, fill=(255, 255, 255))
    return img


def start(
    get_lang: Callable[[], str],
    set_lang: Callable[[str], None],
    on_quit: Callable[[], None],
    on_settings: Callable[[], None] | None = None,
) -> threading.Thread | None:
    try:
        import pystray
    except Exception as e:
        log.warning("pystray unavailable: %s", e)
        return None

    def make_icon():
        lang = get_lang()
        bg = (40, 100, 180) if lang == "en" else (180, 60, 60)
        return _icon(lang.upper(), bg)

    def on_en(icon, item):
        set_lang("en")
        icon.icon = _icon("EN", (40, 100, 180))
        icon.title = "math-speak [EN]"

    def on_fr(icon, item):
        set_lang("fr")
        icon.icon = _icon("FR", (180, 60, 60))
        icon.title = "math-speak [FR]"

    def settings_cb(icon, item):
        if on_settings:
            threading.Thread(target=on_settings, daemon=True).start()

    def quit_cb(icon, item):
        on_quit()
        icon.stop()

    menu_items = [
        pystray.MenuItem("English", on_en, checked=lambda _i: get_lang() == "en"),
        pystray.MenuItem("Français", on_fr, checked=lambda _i: get_lang() == "fr"),
        pystray.Menu.SEPARATOR,
    ]
    if on_settings is not None:
        menu_items.append(pystray.MenuItem("Settings…", settings_cb))
    menu_items.extend([
        pystray.MenuItem("Quit", quit_cb),
    ])
    menu = pystray.Menu(*menu_items)
    icon = pystray.Icon("math-speak", make_icon(), f"math-speak [{get_lang().upper()}]", menu)

    t = threading.Thread(target=icon.run, daemon=True, name="math-speak-tray")
    t.start()
    return t
