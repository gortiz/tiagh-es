#!/usr/bin/env python3
"""
extract_translations.py - Extracts game strings from PCK and organizes translation files.

Gathers all strings across EN, FR, DE, JP, ZHCN, merges them, and creates/updates
structured JSON translation files with English, French, and German reference texts.
Preserves any existing Spanish translations when re-run.
"""

import json
import os
import sys
from typing import Dict, List, Tuple, Any

from godot_pck import GodotPCK
from godot_translation import parse_translation_bytes

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(BASE_DIR, "data")
SOURCE_DIR = os.path.join(DATA_DIR, "source")
TRANSLATIONS_DIR = os.path.join(DATA_DIR, "translations")


def get_first_string(val: Any) -> str:
    if isinstance(val, list):
        return val[0] if val else ""
    return str(val) if val is not None else ""


def categorize_key(key: str) -> str:
    """Categorizes a key into one of the translation files."""
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

    # Everything else is UI, menus, settings, interface, prompts, interactions
    return "ui"


def load_existing_translations(file_path: str) -> Dict[Tuple[str, str], str]:
    """Loads existing Spanish translations if the file already exists."""
    existing = {}
    if os.path.exists(file_path):
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                items = json.load(f)
                for item in items:
                    ctx = item.get("context", "")
                    k = item.get("key", "")
                    es = item.get("es", "")
                    if es:
                        existing[(ctx, k)] = es
        except Exception as e:
            print(f"Warning: Failed to read existing {file_path}: {e}")
    return existing


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

    langs = ["en", "fr", "de", "jp", "zhcn"]
    raw_translations = {}

    for lang in langs:
        pck_file = f"text/galley.{lang}.translation"
        if pck.has_file(pck_file):
            print(f"Extracting {pck_file}...")
            parsed = parse_translation_bytes(pck.extract_file(pck_file))
            raw_translations[lang] = parsed["messages"]
        else:
            print(f"Warning: {pck_file} not in PCK")
            raw_translations[lang] = {}

    # Union of all keys across all languages
    all_keys = set()
    for lang in langs:
        all_keys.update(raw_translations[lang].keys())

    print(f"Total unique (context, key) pairs: {len(all_keys)}")

    os.makedirs(SOURCE_DIR, exist_ok=True)
    os.makedirs(TRANSLATIONS_DIR, exist_ok=True)

    # Save raw archive in data/source/all_strings.json
    archive_data = []
    for ctx, key in sorted(all_keys, key=lambda x: (x[1], x[0])):
        archive_data.append({
            "key": key,
            "context": ctx,
            "en": get_first_string(raw_translations["en"].get((ctx, key))),
            "fr": get_first_string(raw_translations["fr"].get((ctx, key))),
            "de": get_first_string(raw_translations["de"].get((ctx, key))),
            "jp": get_first_string(raw_translations["jp"].get((ctx, key))),
            "zhcn": get_first_string(raw_translations["zhcn"].get((ctx, key))),
        })

    with open(os.path.join(SOURCE_DIR, "all_strings.json"), "w", encoding="utf-8") as f:
        json.dump(archive_data, f, ensure_ascii=False, indent=2)
    print(f"Saved full raw archive: {os.path.join(SOURCE_DIR, 'all_strings.json')}")

    # Group into categories
    categorized_items = {
        "ui": [],
        "rooms": [],
        "evidence": [],
        "hints": [],
        "credits": [],
        "dialogue": []
    }

    # Load existing translations to preserve progress
    existing_es = {}
    for cat in categorized_items.keys():
        dest_file = os.path.join(TRANSLATIONS_DIR, f"{cat}.json")
        existing_es.update(load_existing_translations(dest_file))

    for item in archive_data:
        cat = categorize_key(item["key"])
        ctx = item["context"]
        k = item["key"]
        
        # Use existing Spanish translation if available
        es_val = existing_es.get((ctx, k), "")
        
        entry = {
            "key": k,
            "context": ctx,
            "en": item["en"],
            "fr": item["fr"],
            "de": item["de"],
            "es": es_val
        }
        categorized_items[cat].append(entry)

    # Write categorized files
    print("\nCategorized files summary:")
    for cat, items in categorized_items.items():
        dest_file = os.path.join(TRANSLATIONS_DIR, f"{cat}.json")
        with open(dest_file, "w", encoding="utf-8") as f:
            json.dump(items, f, ensure_ascii=False, indent=2)
        translated = sum(1 for x in items if x["es"].strip())
        print(f"  {cat}.json: {len(items)} keys ({translated} translated) -> {dest_file}")

    print("\nExtraction complete!")


if __name__ == "__main__":
    main()
