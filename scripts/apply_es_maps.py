#!/usr/bin/env python3
"""
apply_es_maps.py - Reads lightweight {key: es_translation} maps from data/chunks/batch_XX_es_map.json
and applies them to data/chunks/batch_XX.json.
"""

import glob
import json
import os
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CHUNKS_DIR = os.path.join(BASE_DIR, "data", "chunks")


def main():
    map_files = sorted(glob.glob(os.path.join(CHUNKS_DIR, "batch_*_es_map.json")))
    if not map_files:
        print("No map files found.")
        return

    for mf in map_files:
        bf = mf.replace("_es_map.json", ".json")
        if not os.path.exists(bf):
            print(f"Warning: Base batch file {bf} not found.")
            continue

        with open(mf, "r", encoding="utf-8") as f:
            es_map = json.load(f)

        with open(bf, "r", encoding="utf-8") as f:
            batch_data = json.load(f)

        applied = 0
        for item in batch_data:
            k = item["key"]
            if k in es_map and es_map[k].strip():
                item["es"] = es_map[k].strip()
                applied += 1

        with open(bf, "w", encoding="utf-8") as f:
            json.dump(batch_data, f, ensure_ascii=False, indent=2)

        print(f"Applied {applied}/{len(batch_data)} translations from {os.path.basename(mf)} to {os.path.basename(bf)}")


if __name__ == "__main__":
    main()
