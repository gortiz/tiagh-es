#!/usr/bin/env python3
"""
translation_batches.py - Splits untranslated keys into batches for a translation agent,
and merges the agent's answers back.

  export [--size N]  Writes data/chunks/batch_XX.json with the keys that have no Spanish text
                     yet: [{key, en, fr, de}]. Needs the local reference texts.
  apply              Reads every data/chunks/batch_XX_es_map.json ({key: Spanish text}) and
                     adds the translations to data/translations/<category>.json.

data/chunks/ holds game texts, so git ignores it.
"""

import argparse
import glob
import json
import os
import shutil

from translation_data import BASE_DIR, CATEGORIES, categorize_key, load_es, load_reference, save_es, source_text

CHUNKS_DIR = os.path.join(BASE_DIR, "data", "chunks")


def export(size: int) -> None:
    pending = []
    for cat in CATEGORIES:
        es = load_es(cat)
        pending += [it for it in load_reference(cat) if it["key"] not in es and source_text(it).strip()]
    if os.path.isdir(CHUNKS_DIR):
        shutil.rmtree(CHUNKS_DIR)
    os.makedirs(CHUNKS_DIR)
    for n, start in enumerate(range(0, len(pending), size)):
        path = os.path.join(CHUNKS_DIR, f"batch_{n:02d}.json")
        with open(path, "w", encoding="utf-8") as f:
            json.dump(pending[start:start + size], f, ensure_ascii=False, indent=2)
    print(f"Exported {len(pending)} untranslated keys in {-(-len(pending) // size)} batches to {CHUNKS_DIR}")


def apply() -> None:
    by_cat = {cat: load_es(cat) for cat in CATEGORIES}
    applied = 0
    for path in sorted(glob.glob(os.path.join(CHUNKS_DIR, "batch_*_es_map.json"))):
        with open(path, "r", encoding="utf-8") as f:
            es_map = json.load(f)
        for key, text in es_map.items():
            if text.strip():
                by_cat[categorize_key(key)][key] = text.strip()
                applied += 1
    for cat, es in by_cat.items():
        save_es(cat, es, order=[it["key"] for it in load_reference(cat)])
    print(f"Applied {applied} translations.")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    exp = sub.add_parser("export")
    exp.add_argument("--size", type=int, default=100)
    sub.add_parser("apply")
    args = parser.parse_args()
    if args.command == "export":
        export(args.size)
    else:
        apply()


if __name__ == "__main__":
    main()
