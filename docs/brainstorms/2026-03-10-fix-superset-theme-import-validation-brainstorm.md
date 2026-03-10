---
title: "Fix Superset Theme Import Validation Error"
type: fix
date: 2026-03-10
status: complete
---

# Fix: Superset Theme Import "Invalid input" Error

## What's Broken

When importing a chromaport-generated theme JSON into Superset via Settings > Appearance > Import Theme, the app shows:

> **Failed to import theme file**
> Theme 1: Invalid input

## Root Cause

**`cursorAccent: null` fails Zod validation.**

Superset's import schema (`apps/desktop/src/shared/themes/import.ts`) uses:

```typescript
cursorAccent: z.string().optional(),      // accepts: string | undefined
selectionBackground: z.string().optional(), // accepts: string | undefined
```

Zod's `.optional()` means `string | undefined` — it does **NOT** accept `null`.

In JSON:
- `undefined` = field is absent (key not present)
- `null` = field is present with value `null`

chromaport's `ir_to_json` outputs:

```json
"cursorAccent": null,
"selectionBackground": "#80CBC420"
```

When `cursor_accent` is `None` in Rust, `Option::None` serializes to JSON `null` via `serde_json::json!`. This `null` value fails `z.string().optional()`.

## Why This Approach

### Chosen Fix: Omit `None` fields from JSON output

Instead of emitting `null` for optional terminal color fields, simply don't include the key in the JSON. This matches what Superset expects — absent fields fall through to defaults via `getDefaultTerminalColors(type)`:

```typescript
// import.ts:163-166
terminal: {
    ...getDefaultTerminalColors(type),  // defaults
    ...(terminalOverrides ?? {}),       // our overrides merge on top
},
```

### Why not fix Superset's schema instead?

Could change `.optional()` to `.nullable().optional()`, but:
1. chromaport should output valid import format regardless
2. Other theme files (manual or third-party) would also omit null fields
3. Omitting is the JSON convention for "not specified"

### Implementation approach: Dynamic Map construction

`serde_json::json!` macro creates a static structure — can't conditionally omit fields. Need to build the `terminal` object as a `serde_json::Map` and only insert fields that have values:

```rust
let mut terminal = serde_json::Map::new();
terminal.insert("background".into(), json!(t.background.as_str()));
// ... required fields ...

// Optional fields: only insert if Some
if let Some(ref c) = t.cursor_accent {
    terminal.insert("cursorAccent".into(), json!(c.as_str()));
}
if let Some(ref c) = t.selection_bg {
    terminal.insert("selectionBackground".into(), json!(c.as_str()));
}
```

## Key Decisions

1. **Fix in chromaport, not Superset** — chromaport should produce valid import files
2. **Omit null fields, don't serialize as null** — matches JSON convention and Zod `.optional()` semantics
3. **Only terminal block affected** — all `ui` fields come from required `ThemeIR` fields (no `Option`)
4. **Scope**: `cursorAccent` and `selectionBackground` are the only two `Option<HexColor>` fields in `AnsiColors`

## Open Questions

None — root cause is confirmed, fix approach is clear.
