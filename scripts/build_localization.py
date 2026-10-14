#!/usr/bin/env python3
"""
build_localization.py - Compiles the Spanish translations into build/galley.es.translation.

Only translated keys go into the file. For a key without Spanish text, Godot shows the text
of the fallback locale (English).
"""

import os

from godot_translation import save_translation_bytes
from translation_data import BASE_DIR, CATEGORIES, load_es

BUILD_DIR = os.path.join(BASE_DIR, "build")


def main():
    messages = {}
    for cat in CATEGORIES:
        es = load_es(cat)
        print(f"{cat}: {len(es)} translations")
        for key, text in es.items():
            messages[("", key)] = [text]

    os.makedirs(BUILD_DIR, exist_ok=True)
    out_file = os.path.join(BUILD_DIR, "galley.es.translation")
    raw = save_translation_bytes(messages, locale="es")
    with open(out_file, "wb") as f:
        f.write(raw)
    print(f"\nSaved {out_file} ({len(messages):,} keys, {len(raw):,} bytes)")


if __name__ == "__main__":
    main()
