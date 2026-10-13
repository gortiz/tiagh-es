#!/usr/bin/env python3
"""
validate_tags.py - Validates BBCode tags, variable tokens, and placeholders between
source and translated strings to prevent game crashes or UI glitches.
"""

import json
import os
import re
import sys
from typing import Dict, List, Set, Tuple

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TRANSLATIONS_DIR = os.path.join(BASE_DIR, "data", "translations")

BBCODE_PATTERN = re.compile(r"\[/?([a-zA-Z0-9_=#]+)\]")
PLACEHOLDER_PATTERN = re.compile(r"\{[a-zA-Z0-9_]+\}")
PRINTF_PATTERN = re.compile(r"%[0-9]*\.?[0-9]*[sdif]")


def extract_features(text: str) -> Tuple[List[str], List[str], List[str]]:
    bbcode = BBCODE_PATTERN.findall(text)
    # Normalize bbcode tag names (strip arguments)
    norm_bbcode = [tag.split("=")[0].lower() for tag in bbcode]
    placeholders = PLACEHOLDER_PATTERN.findall(text)
    printf = PRINTF_PATTERN.findall(text)
    return norm_bbcode, sorted(placeholders), sorted(printf)


def check_balanced_bbcode(text: str) -> List[str]:
    errors = []
    stack = []
    # Find all opening and closing tags
    tags = re.findall(r"\[(/?[a-zA-Z0-9_=#]+)\]", text)
    for tag in tags:
        tag_name = tag.split("=")[0].lower()
        if tag_name.startswith("/"):
            close_name = tag_name[1:]
            if not stack:
                errors.append(f"Closing tag [/{close_name}] without opening tag")
            elif stack[-1] != close_name:
                errors.append(f"Mismatched closing tag [/{close_name}], expected [/{stack[-1]}]")
            else:
                stack.pop()
        else:
            # Self-closing or special tags? None known in Godot rich text
            stack.append(tag_name)
    if stack:
        errors.append(f"Unclosed BBCode tags: {', '.join('[' + t + ']' for t in stack)}")
    return errors


def validate_file(path: str) -> List[Dict[str, Any]]:
    issues = []
    with open(path, "r", encoding="utf-8") as f:
        items = json.load(f)

    for it in items:
        key = it.get("key", "")
        src = it.get("en", "") or it.get("fr", "")
        trans = it.get("es", "").strip()

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
    if not os.path.exists(TRANSLATIONS_DIR):
        print("Translations directory not found.")
        sys.exit(1)

    files = sorted([f for f in os.listdir(TRANSLATIONS_DIR) if f.endswith(".json")])
    total_issues = 0

    print("Running QA tag and placeholder validation...")
    for fn in files:
        path = os.path.join(TRANSLATIONS_DIR, fn)
        issues = validate_file(path)
        if issues:
            print(f"\n[!] Issues in {fn} ({len(issues)} found):")
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
