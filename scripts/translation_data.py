"""
translation_data.py - Shared access to the Spanish translations and the local reference texts.

The repository stores only the Spanish text: data/translations/<category>.json maps each
message key to its translation. The original texts (en, fr, de) belong to the game, so they
are not in the repository. scripts/extract_translations.py copies them from your own game
installation into data/reference/<category>.json, which git ignores.
"""

import json
import os
import sys
from typing import Dict, List

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TRANSLATIONS_DIR = os.path.join(BASE_DIR, "data", "translations")
REFERENCE_DIR = os.path.join(BASE_DIR, "data", "reference")

CATEGORIES = ["ui", "rooms", "evidence", "hints", "credits", "dialogue"]


def categorize_key(key: str) -> str:
    """Returns the category file for a message key."""
    if key.startswith("room.") or key.startswith("code.") or key in (
        "extra_calibration_chars", "katherine_char", "room_characters"
    ):
        return "rooms"
    if key.startswith("evidence.") or key.startswith("db.ev."):
        return "evidence"
    if key.startswith("db.hint."):
        return "hints"
    if key.startswith("db.credit."):
        return "credits"
    if key.startswith("db.script.") or key.startswith("db."):
        return "dialogue"
    return "ui"


def load_es(category: str) -> Dict[str, str]:
    """Returns {key: Spanish text} for one category."""
    path = os.path.join(TRANSLATIONS_DIR, f"{category}.json")
    if not os.path.exists(path):
        return {}
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def save_es(category: str, translations: Dict[str, str], order: List[str] = None) -> None:
    """Writes {key: Spanish text} for one category, in reference order when given."""
    if order:
        ranked = {key: i for i, key in enumerate(order)}
        items = sorted(translations.items(), key=lambda kv: (ranked.get(kv[0], len(ranked)), kv[0]))
    else:
        items = list(translations.items())
    path = os.path.join(TRANSLATIONS_DIR, f"{category}.json")
    os.makedirs(TRANSLATIONS_DIR, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump({k: v for k, v in items if v.strip()}, f, ensure_ascii=False, indent=2)
        f.write("\n")


def load_reference(category: str) -> List[Dict[str, str]]:
    """Returns the original texts of one category: [{key, en, fr, de}].

    Exits with a message when the reference is missing.
    """
    path = os.path.join(REFERENCE_DIR, f"{category}.json")
    if not os.path.exists(path):
        print(
            f"Error: {path} not found.\n"
            "Copy the original texts from your game first:\n"
            "  python3 scripts/extract_translations.py <path/to/game.pck>"
        )
        sys.exit(1)
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def source_text(item: Dict[str, str]) -> str:
    """The text to translate from: English, or French/German when English is empty."""
    return item.get("en", "") or item.get("fr", "") or item.get("de", "")
