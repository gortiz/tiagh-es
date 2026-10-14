#!/usr/bin/env python3
"""
extract_translations.py - Copies the original game texts from your game's PCK into
data/reference/<category>.json, for translators and for the QA tools.

The reference holds the English, French and German texts of every message key. It belongs
to the game, so git ignores it. The Spanish translations in data/translations/ do not change.
"""

import json
import os
import sys
from typing import Any

from godot_pck import GodotPCK
from godot_translation import parse_translation_bytes
from translation_data import CATEGORIES, REFERENCE_DIR, categorize_key, load_es

LANGS = ["en", "fr", "de"]


def first_string(val: Any) -> str:
    if isinstance(val, list):
        return val[0] if val else ""
    return str(val) if val is not None else ""


def main():
    if len(sys.argv) != 2:
        print("Usage: python3 scripts/extract_translations.py <path/to/game.pck>")
        sys.exit(1)
    pck_path = sys.argv[1]
    if not os.path.exists(pck_path):
        print(f"Error: PCK not found at {pck_path}")
        sys.exit(1)

    print(f"Loading PCK: {pck_path}")
    pck = GodotPCK(pck_path)

    messages = {}
    for lang in LANGS:
        pck_file = f"text/galley.{lang}.translation"
        if not pck.has_file(pck_file):
            print(f"Error: {pck_file} not in PCK")
            sys.exit(1)
        messages[lang] = parse_translation_bytes(pck.extract_file(pck_file))["messages"]

    keys = set()
    for lang in LANGS:
        keys.update(key for ctx, key in messages[lang].keys() if ctx == "")

    reference = {cat: [] for cat in CATEGORIES}
    for key in sorted(keys):
        item = {"key": key}
        item.update({lang: first_string(messages[lang].get(("", key))) for lang in LANGS})
        reference[categorize_key(key)].append(item)

    os.makedirs(REFERENCE_DIR, exist_ok=True)
    print(f"\n{len(keys)} message keys:")
    for cat, items in reference.items():
        with open(os.path.join(REFERENCE_DIR, f"{cat}.json"), "w", encoding="utf-8") as f:
            json.dump(items, f, ensure_ascii=False, indent=2)
        es = load_es(cat)
        missing = sum(1 for it in items if it["key"] not in es)
        stale = len(set(es) - {it["key"] for it in items})
        print(f"  {cat}: {len(items)} keys, {missing} without Spanish, {stale} Spanish keys no longer in the game")

    print(f"\nReference saved in {REFERENCE_DIR}")


if __name__ == "__main__":
    main()
