# Research "house format" — one-click imports

The renderer's **➕ Add Research** button (and `nbe note-import <file.md>`) reads a small header off
the top of a markdown file to set the note's **title** and **topic tags**. If you ask Gemini (or any
tool) to produce research in this format, importing is one click and the note auto-files itself under
the right topic hubs in the Research region of the brain.

## The format

Either of these two headers works. **Front-matter** (preferred):

```markdown
---
title: Creatine for strength clients
tags: Nutrition, Supplements, Strength
---

Creatine monohydrate is the most evidence-backed supplement for...
```

…or **leading `Title:` / `Tags:` lines** (must be the very first non-blank lines, ended by a blank line):

```markdown
Title: Creatine for strength clients
Tags: Nutrition, Supplements, Strength

Creatine monohydrate is the most evidence-backed supplement for...
```

### Rules (exactly what the importer does)
- **`tags`** are comma-separated. Surrounding `[ ]` and quotes are stripped; each becomes a **topic
  hub** the note links to (created if new), so tagged notes cluster together. Tags are de-duplicated
  case-insensitively, and merged with any topics passed at import time.
- **`title`** sets the note name. If omitted, the importer falls back to the first `# Heading`, then
  the file name. The stored body always begins with exactly one `# {title}` line (a duplicate leading
  H1 in the body is dropped), so you don't need to repeat the title as a heading.
- No header at all is fine — you just get a note titled from the first heading / filename and no tags
  (you can tag it later in the app or with `note-tag`).

## A prompt you can give Gemini

> Produce a research note as a markdown file. Begin the file with YAML front-matter containing
> `title:` (a short descriptive title) and `tags:` (a comma-separated list of 1–4 broad topics such
> as Nutrition, Training, Recovery, Programming, Business). Then write the note body in markdown.
> Do not repeat the title as a heading in the body. Keep it focused and skimmable.

Save Gemini's output as a `.md` file, click **➕ Add Research**, pick the file — the note and its topic
hubs appear in the brain immediately.
