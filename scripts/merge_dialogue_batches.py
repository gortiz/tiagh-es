#!/usr/bin/env python3
"""
merge_dialogue_batches.py - Merges translated batches from data/chunks/ into data/translations/dialogue.json
"""

import glob
import json
import os
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIALOGUE_PATH = os.path.join(BASE_DIR, "data", "translations", "dialogue.json")
CHUNKS_DIR = os.path.join(BASE_DIR, "data", "chunks")


def main():
    if not os.path.exists(DIALOGUE_PATH):
        print(f"Error: {DIALOGUE_PATH} not found.")
        sys.exit(1)

    with open(DIALOGUE_PATH, "r", encoding="utf-8") as f:
        dialogue = json.load(f)

    # Build key index for fast lookup
    dialogue_map = {item["key"]: item for item in dialogue}

    batch_files = sorted(glob.glob(os.path.join(CHUNKS_DIR, "batch_[0-9][0-9].json")))
    if not batch_files:
        print(f"No batch files found in {CHUNKS_DIR}.")
        sys.exit(0)

    total_merged = 0
    total_in_batches = 0
    completed_batches = 0

    for bf in batch_files:
        with open(bf, "r", encoding="utf-8") as f:
            batch_items = json.load(f)

        batch_completed = True
        for item in batch_items:
            k = item["key"]
            es = item.get("es", "").strip()
            total_in_batches += 1
            if es:
                if k in dialogue_map:
                    dialogue_map[k]["es"] = es
                    total_merged += 1
            else:
                batch_completed = False

        if batch_completed and len(batch_items) > 0:
            completed_batches += 1

    with open(DIALOGUE_PATH, "w", encoding="utf-8") as f:
        json.dump(dialogue, f, ensure_ascii=False, indent=2)

    print(f"Merged {total_merged} translations from {len(batch_files)} batches.")
    print(f"Completed batches: {completed_batches} / {len(batch_files)}")


if __name__ == "__main__":
    main()
