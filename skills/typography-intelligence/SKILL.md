---
name: typography-intelligence
description: Plan readable typographic hierarchy, measure text with approved local fonts and diagnose crowded document or presentation layouts.
metadata:
  version: "1.0"
---
# Typography Intelligence

Use this skill for font sizing, hierarchy, text fit and readability. Load it before design_tool type_scale or measure_text. Never run arbitrary scripts or install fonts.

1. Identify medium, audience, reading distance, language and any supplied brand constraints.
2. Choose a restrained font system. Start with one family and distinguish roles by size and weight. Pair fonts only when their contrast serves a purpose; do not invent installed font availability.
3. Request type_scale with mode document or presentation and base size in points. For documents start around 11 pt; for projected slides prefer 20–28 pt where feasible.
4. Use measure_text for representative text in the available width in points. Windows uses Arial and Linux uses Liberation Sans in the reviewed runtime. Measurements are estimates, not native layout guarantees.
5. Increase available space or split content before shrinking text. Avoid excessively long lines, crowded paragraphs, isolated headings and inconsistent spacing.
6. Pass only supported typography tokens to compose_v2 (font, body_size, slide_body_size); other scale values are guidance.
7. Render and review every page. Report unavailable glyph coverage, font substitution or visual review honestly. Font licensing and multilingual shaping require separate verification.
