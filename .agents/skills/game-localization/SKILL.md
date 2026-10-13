---
name: game-localization
description: >-
  High-fidelity, token-efficient video game localization workflow for AI agents.
  Specialized in reverse-engineering game text, batch translation using lightweight
  key-value mapping, QA tag validation, and building binary translation packs.
---

# Game Localization Skill: Token-Efficient Pipeline

This skill defines the standard procedure for extracting, translating, validating, and building video game localizations (Godot, Unity, Unreal, Ren'Py, etc.) with minimal token overhead and maximum literary quality.

---

## 1. Core Principles

1. **Lightweight Key-Value Output Pattern (Token Optimization)**:
   - When translating dialogue or multi-language projects, **NEVER** ask the translation agent to rewrite or echo back existing fields (like `en`, `fr`, `de`, `context`).
   - The agent reads the multi-language context as input, but outputs **ONLY** a flat JSON dictionary:
     ```json
     {
       "msg_id_001": "Traducción optimizada al idioma destino...",
       "msg_id_002": "Segunda traducción..."
     }
     ```
   - **Savings**: Reduces generated output tokens by 70–80%, speeds up execution by 4x, and avoids rate limits or context truncation.
   - A deterministic local Python script merges this map back into the master dataset.

2. **Literary Consistency & Natural Registers**:
   - Maintain appropriate interpersonal registers (e.g. Spanish *tuteo* for intimates, *usted* for formal/strangers).
   - Preserve tone (British noir, comedy, sci-fi, fantasy) without literal translation calques.
   - Pay strict attention to detective clues, ambiguous descriptions, sound cues, and gender neutrality when present in the original.

3. **Engine Tag & Placeholder Integrity**:
   - Tags (`[room]...[/room]`, `[sound]...[/sound]`, `[b]...[/b]`, `[i]...[/i]`, `[color=...]...[/color]`, `{0}`, `%s`) must never be broken or mismatched.
   - Standardized validator script checks all opening/closing tags against the original string before compilation.

---

## 2. Standard Workflow & Automation

### Step 1: Chunking & Preparation
Split untranslated strings into manageable batches (typically 80–120 entries per batch):
```python
# scripts/export_batches.py
BATCH_SIZE = 100
# Slices sorted by conversation/scene flow for narrative continuity
```

### Step 2: Agent Prompt Template (Lightweight Map)
When delegating batches to subagents:
```text
Task:
1. Read input batch: {batch_file}
2. For each key, translate 'en' into target language, using available reference languages ('fr', 'de') for register/gender clarification.
3. Preserve all BBCode tags and placeholders.
4. OUTPUT ONLY a JSON map: { "key": "translated_text" }
5. Save as: {batch_es_map_file}
```

### Step 3: Map Application & Master Merge
Use a simple script to apply the flat maps back into the dataset:
```python
# scripts/apply_es_maps.py
for map_file in glob.glob("data/chunks/batch_*_es_map.json"):
    base_file = map_file.replace("_es_map.json", ".json")
    # merge flat dict into base json list
```

And merge into master translation file:
```python
# scripts/merge_dialogue_batches.py
```

### Step 4: Tag & Placeholder QA Validation
Always run automated syntax checking before building:
```bash
python3 scripts/validate_tags.py
```
Ensures:
- No unclosed BBCode tags.
- Identical tag counts between source and target.
- Untranslated critical IDs inside attributes (e.g. `[room id=AT]`).

### Step 5: Compilation & Release Packaging
- Compile translation into engine-native binary format (e.g., Godot `.translation` resource via `godot_translation.py`, or gettext `.mo`).
- Package into standalone mod patch or release zip with an automated installer script.
