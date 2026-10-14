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

### Step 0: Reference Texts (Local Only)
The repository stores only the Spanish text (`data/translations/<category>.json`, a flat `{key: text}` map).
The original texts belong to the game and must never be committed. Copy them from a local game installation:
```bash
python3 scripts/extract_translations.py "<path/to/game.pck>"   # writes data/reference/ (gitignored)
```

### Step 1: Chunking & Preparation
Split untranslated keys into batches (100 entries by default):
```bash
python3 scripts/translation_batches.py export --size 100   # writes data/chunks/batch_XX.json (gitignored)
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

### Step 3: Merge
Merge every `data/chunks/batch_XX_es_map.json` into `data/translations/`:
```bash
python3 scripts/translation_batches.py apply
```

### Step 4: Tag & Placeholder QA Validation
Always run automated syntax checking before building:
```bash
python3 scripts/validate_tags.py
python3 scripts/check_status.py
```
Ensures:
- No unclosed BBCode tags.
- Identical tag counts between source and target.
- Untranslated critical IDs inside attributes (e.g. `[room id=AT]`).

### Step 5: Compilation & Release Packaging
- Compile the translation into Godot's binary `.translation` format: `python3 scripts/build_localization.py`.
- The Rust installer in `installer/` embeds it. Game scripts are patched at install time from the player's own copy (`installer/src/scripts.rs`); never commit game code.
