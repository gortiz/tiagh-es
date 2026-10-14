#!/usr/bin/env python3
"""
check_status.py - Translation coverage report. Needs the local reference texts
(see scripts/extract_translations.py).
"""

from translation_data import CATEGORIES, load_es, load_reference, source_text


def count_words(text: str) -> int:
    return len(text.split())


def main():
    rows = []
    for cat in CATEGORIES:
        es = load_es(cat)
        items = load_reference(cat)
        total_keys = trans_keys = total_words = trans_words = 0
        for it in items:
            words = count_words(source_text(it))
            total_keys += 1
            total_words += words
            # Keys with no source text need no translation.
            if it["key"] in es or not source_text(it).strip():
                trans_keys += 1
                trans_words += words
        rows.append((f"{cat}.json", trans_keys, total_keys, trans_words, total_words))

    print("=" * 80)
    print(f"{'Archivo':<18} | {'Claves':<12} | {'Progreso (%)':<14} | {'Palabras':<14} | {'Estado':<10}")
    print("=" * 80)
    for name, tk, k, tw, w in rows:
        pct = tk / k * 100 if k else 0.0
        status = "COMPLETO" if tk == k and k else ("EN CURSO" if tk else "PENDIENTE")
        print(f"{name:<18} | {f'{tk:,} / {k:,}':<12} | {f'{pct:6.2f}%':<14} | {f'{tw:,} / {w:,}':<14} | {status:<10}")
    tk, k, tw, w = (sum(r[i] for r in rows) for i in range(1, 5))
    pct = tk / k * 100 if k else 0.0
    print("=" * 80)
    print(f"{'TOTAL GENERAL':<18} | {f'{tk:,} / {k:,}':<12} | {f'{pct:6.2f}%':<14} | {f'{tw:,} / {w:,}':<14} |")
    print("=" * 80)


if __name__ == "__main__":
    main()
