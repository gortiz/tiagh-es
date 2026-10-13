#!/usr/bin/env python3
"""
build_localization.py - Compiles Spanish translation and script assets into build artifacts.
"""

import json
import os
import subprocess
import sys
from typing import Dict, List, Tuple

from godot_translation import save_translation_bytes

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(BASE_DIR, "data")
TRANSLATIONS_DIR = os.path.join(DATA_DIR, "translations")
BUILD_DIR = os.path.join(BASE_DIR, "build")
SRC_DIR = os.path.join(BASE_DIR, "src")
TOOLS_DIR = os.path.join(BASE_DIR, "tools")
GDRE_BIN = os.path.join(TOOLS_DIR, "gdre_tools.x86_64")


def compile_scripts():
    print("\n--- Compiling scripts ---")
    scripts = [
        os.path.join(SRC_DIR, "scripts", "game-config.gd"),
        os.path.join(SRC_DIR, "scripts", "lang-consts.gd"),
    ]
    env = os.environ.copy()
    env["LD_LIBRARY_PATH"] = TOOLS_DIR

    for s in scripts:
        if not os.path.exists(s):
            print(f"Error: Script {s} not found!")
            sys.exit(1)
        print(f"Compiling {os.path.basename(s)}...")
        res = subprocess.run([
            GDRE_BIN, "--headless",
            f"--compile={s}",
            "--bytecode=4.6.2",
            f"--output={BUILD_DIR}"
        ], env=env, capture_output=True, text=True)

        if res.returncode != 0:
            print(f"Compilation error for {s}:\n{res.stderr}\n{res.stdout}")
            sys.exit(1)


def create_remaps():
    print("\n--- Generating resource remaps ---")
    remaps = {
        "resources/localized-textures-es.tres.remap": (
            "[remap]\n\n"
            'path="res://.godot/exported/2878949781/export-37bb29907cde80b785461ecdabf2a7da-localized-textures-en.res"\n'
        ),
        "db/search-index-es.tres.remap": (
            "[remap]\n\n"
            'path="res://.godot/exported/2878949781/export-b58cc7bd8a815806609785fe75ac971a-search-index-en.res"\n'
        ),
    }

    for rel_path, content in remaps.items():
        dest = os.path.join(BUILD_DIR, os.path.basename(rel_path))
        with open(dest, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"Generated {os.path.basename(dest)}")


def build_translation():
    print("\n--- Building galley.es.translation ---")
    files = sorted([f for f in os.listdir(TRANSLATIONS_DIR) if f.endswith(".json")])

    messages: Dict[Tuple[str, str], List[str]] = {}
    total_keys = 0
    translated_keys = 0

    for fn in files:
        path = os.path.join(TRANSLATIONS_DIR, fn)
        with open(path, "r", encoding="utf-8") as f:
            items = json.load(f)

        for it in items:
            total_keys += 1
            ctx = it.get("context", "")
            key = it.get("key", "")
            es_text = it.get("es", "").strip()

            if es_text:
                translated_keys += 1
                final_text = es_text
            else:
                # Fallback to English (or French/German if EN is empty)
                final_text = it.get("en", "") or it.get("fr", "") or it.get("de", "")

            messages[(ctx, key)] = [final_text]

    print(f"Assembled {len(messages)} keys ({translated_keys} translated, {total_keys - translated_keys} fallbacks).")

    raw_translation = save_translation_bytes(messages, locale="es")
    out_file = os.path.join(BUILD_DIR, "galley.es.translation")
    with open(out_file, "wb") as f:
        f.write(raw_translation)

    print(f"Saved binary translation: {out_file} ({len(raw_translation):,} bytes)")
    return total_keys, translated_keys


def main():
    os.makedirs(BUILD_DIR, exist_ok=True)

    total_keys, trans_keys = build_translation()
    compile_scripts()
    create_remaps()

    pct = (trans_keys / total_keys * 100) if total_keys else 0.0
    print("\n" + "=" * 60)
    print("BUILD SUCCESSFUL!")
    print(f"Spanish Translation Coverage: {trans_keys:,} / {total_keys:,} ({pct:.2f}%)")
    print(f"Artifacts ready in: {BUILD_DIR}")
    print("=" * 60)


if __name__ == "__main__":
    main()
