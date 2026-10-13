#!/usr/bin/env python3
"""
check_status.py - Translation coverage and statistics report.
"""

import json
import os
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TRANSLATIONS_DIR = os.path.join(BASE_DIR, "data", "translations")


def count_words(text: str) -> int:
    return len(text.strip().split()) if text.strip() else 0


def main():
    if not os.path.exists(TRANSLATIONS_DIR):
        print(f"Error: Translations directory not found at {TRANSLATIONS_DIR}")
        sys.exit(1)

    files = sorted([f for f in os.listdir(TRANSLATIONS_DIR) if f.endswith(".json")])
    if not files:
        print("No translation files found.")
        sys.exit(1)

    grand_total_keys = 0
    grand_trans_keys = 0
    grand_total_words = 0
    grand_trans_words = 0

    print("=" * 80)
    print(f"{'Archivo':<18} | {'Claves':<12} | {'Progreso (%)':<14} | {'Palabras':<14} | {'Estado':<10}")
    print("=" * 80)

    for fn in files:
        path = os.path.join(TRANSLATIONS_DIR, fn)
        with open(path, "r", encoding="utf-8") as f:
            items = json.load(f)

        total_keys = len(items)
        trans_keys = 0
        total_words = 0
        trans_words = 0

        for it in items:
            en_text = it.get("en", "") or it.get("fr", "") or it.get("de", "")
            es_text = it.get("es", "").strip()

            words = count_words(en_text)
            total_words += words

            if es_text or not en_text.strip():
                trans_keys += 1
                trans_words += words

        grand_total_keys += total_keys
        grand_trans_keys += trans_keys
        grand_total_words += total_words
        grand_trans_words += trans_words

        pct_keys = (trans_keys / total_keys * 100) if total_keys else 0.0
        status = "COMPLETO" if trans_keys == total_keys and total_keys > 0 else (
            "EN CURSO" if trans_keys > 0 else "PENDIENTE"
        )

        keys_str = f"{trans_keys:,} / {total_keys:,}"
        pct_str = f"{pct_keys:6.2f}%"
        words_str = f"{trans_words:,} / {total_words:,}"

        print(f"{fn:<18} | {keys_str:<12} | {pct_str:<14} | {words_str:<14} | {status:<10}")

    print("=" * 80)
    total_pct = (grand_trans_keys / grand_total_keys * 100) if grand_total_keys else 0.0
    total_keys_str = f"{grand_trans_keys:,} / {grand_total_keys:,}"
    total_pct_str = f"{total_pct:6.2f}%"
    total_words_str = f"{grand_trans_words:,} / {grand_total_words:,}"

    print(f"{'TOTAL GENERAL':<18} | {total_keys_str:<12} | {total_pct_str:<14} | {total_words_str:<14} |")
    print("=" * 80)


if __name__ == "__main__":
    main()
