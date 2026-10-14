#!/usr/bin/env python3
"""
validate_tags.py - Validates BBCode tags, variable tokens, and placeholders between
source and translated strings to prevent game crashes or UI glitches. Needs the local
reference texts (see scripts/extract_translations.py).
"""

import re
import sys
from typing import Any, Dict, List, Tuple

from translation_data import CATEGORIES, load_es, load_reference, source_text

BBCODE_TAG_PATTERN = re.compile(r"\[(/?)([a-zA-Z0-9_]+)(?:=[^\]]*|\s+[^\]]*)?\]")
PLACEHOLDER_PATTERN = re.compile(r"\{[a-zA-Z0-9_]+\}")
PRINTF_PATTERN = re.compile(r"%[0-9]*\.?[0-9]*[sdif]")

KNOWN_BBCODE_TAGS = {
    "b", "i", "u", "s", "code", "center", "right", "fill", "indent",
    "url", "color", "font", "font_size", "outline_size", "outline_color",
    "table", "cell", "dropcap", "img", "person", "room", "note", "warning",
    "title", "pageno", "pre", "endgame", "enters", "leaves", "sound", "time", "evidence"
}


def extract_features(text: str) -> Tuple[List[str], List[str], List[str]]:
    bbcode = []
    for m in BBCODE_TAG_PATTERN.finditer(text):
        tag_name = m.group(2).lower()
        if tag_name not in KNOWN_BBCODE_TAGS:
            continue
        is_closing = m.group(1) == "/"
        bbcode.append(("/" + tag_name) if is_closing else tag_name)
    placeholders = PLACEHOLDER_PATTERN.findall(text)
    printf = PRINTF_PATTERN.findall(text)
    return bbcode, sorted(placeholders), sorted(printf)


def check_balanced_bbcode(text: str) -> List[str]:
    errors = []
    stack = []
    for m in BBCODE_TAG_PATTERN.finditer(text):
        tag_name = m.group(2).lower()
        if tag_name not in KNOWN_BBCODE_TAGS:
            continue
        is_closing = m.group(1) == "/"
        if is_closing:
            if not stack:
                errors.append(f"Closing tag [/{tag_name}] without opening tag")
            elif stack[-1] != tag_name:
                errors.append(f"Mismatched closing tag [/{tag_name}], expected [/{stack[-1]}]")
            else:
                stack.pop()
        else:
            stack.append(tag_name)
    if stack:
        errors.append(f"Unclosed BBCode tags: {', '.join('[' + t + ']' for t in stack)}")
    return errors


def validate_category(category: str) -> List[Dict[str, Any]]:
    issues = []
    es = load_es(category)
    for it in load_reference(category):
        key = it["key"]
        src = source_text(it)
        trans = es.get(key, "").strip()

        if not trans:
            continue

        src_bb, src_ph, src_pf = extract_features(src)
        dst_bb, dst_ph, dst_pf = extract_features(trans)

        # Check placeholders
        if src_ph != dst_ph:
            issues.append({
                "key": key,
                "type": "PLACEHOLDER_MISMATCH",
                "details": f"Expected {src_ph}, found {dst_ph}",
                "src": src,
                "trans": trans
            })

        # Check printf tokens
        if src_pf != dst_pf:
            issues.append({
                "key": key,
                "type": "PRINTF_MISMATCH",
                "details": f"Expected {src_pf}, found {dst_pf}",
                "src": src,
                "trans": trans
            })

        # Check BBCode balance in translation
        balance_errors = check_balanced_bbcode(trans)
        if balance_errors:
            issues.append({
                "key": key,
                "type": "UNBALANCED_BBCODE",
                "details": "; ".join(balance_errors),
                "src": src,
                "trans": trans
            })

        # Check missing source BBCode tags in translation
        missing_tags = [t for t in src_bb if t not in dst_bb]
        if missing_tags:
            issues.append({
                "key": key,
                "type": "MISSING_BBCODE",
                "details": f"Missing tags: {missing_tags}",
                "src": src,
                "trans": trans
            })

    return issues


def main():
    total_issues = 0

    print("Running QA tag and placeholder validation...")
    for cat in CATEGORIES:
        issues = validate_category(cat)
        if issues:
            print(f"\n[!] Issues in {cat}.json ({len(issues)} found):")
            for iss in issues:
                print(f"  - Key: {iss['key']}")
                print(f"    Type: {iss['type']} ({iss['details']})")
                print(f"    Original:   {iss['src']}")
                print(f"    Translated: {iss['trans']}")
            total_issues += len(issues)

    if total_issues == 0:
        print("\nAll translated tags and placeholders are valid! (0 errors)")
    else:
        print(f"\nTotal errors found: {total_issues}")
        sys.exit(1)


if __name__ == "__main__":
    main()
