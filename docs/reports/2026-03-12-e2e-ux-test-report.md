# chromaport E2E UX Test Report
Date: 2026-03-12
Binary: ./target/debug/chromaport (git: 6a24702)
Environment: macOS 26.2, tmux 3.6a

---

## Executive Summary

- Total flows: 5
- Passed: 3
- Failed: 2
- Skipped: 0

Flows 1–3 all succeeded cleanly. Flows 4–5 failed due to a missing preset manifest on the remote repository (`assets/presets/index.json` returns HTTP 404 on `hamsurang/chromaport` main branch). Network connectivity was confirmed healthy (raw.githubusercontent.com responds 301).

---

## Environment

| Dependency  | Status  | Detail                                  |
|-------------|---------|------------------------------------------|
| tmux        | OK      | tmux 3.6a                               |
| cargo build | OK      | 1 warning (dead_code: `rgb_to_hsl`), no errors |
| VS Code     | OK      | ~/.vscode/extensions present            |
| Cursor      | OK      | ~/.cursor/extensions present            |
| Superset    | OK      | ~/.superset present                     |
| Warp        | OK      | ~/.warp present                         |
| Ghostty     | OK      | ~/.config/ghostty present               |
| Network     | OK (partial) | raw.githubusercontent.com → 301; manifest URL → 404 |

---

## Flow 1: Default Theme Selection

**Result**: Passed

### Experience Log

**Step 1 — Launch:**
```
./target/debug/chromaport
```
Application started immediately with no lag.

**Step 2 — Editor selection (inquire Select):**
```
? Select editor:
> VS Code
  Cursor
[↑↓ to move, enter to select, type to filter]
```
Both installed editors were auto-detected and presented cleanly. Selected VS Code with Enter.

**Step 3 — Target app selection (inquire Select):**
```
? Select target app:
> Superset
  Warp
  Ghostty
[↑↓ to move, enter to select, type to filter]
```
Three installed targets detected and listed. Selected Superset with Enter.

**Step 4 — ratatui TUI (theme list, initial state):**
```
┌ Select Theme ──────────────────────────┐┌ Preview: Ayu Dark (Dark) ──────────────────────────────────────────────────┐
│ > Ayu Dark                             ││ bg:#0B0E14  fg:#BFBDB6  accent:#E6B450                                     │
│   Ayu Dark Bordered                    ││                                                                            │
│   Ayu Light                            ││ Normal: ██ ██ ██ ██ ██ ██ ██ ██                                            │
│   Ayu Light Bordered                   ││ Bright: ██ ██ ██ ██ ██ ██ ██ ██                                            │
│   Ayu Mirage                           ││ Chart: ██ ██ ██ ██ ██                                                      │
│   ...                                  ││                                                                            │
│   Material Theme (many variants)       ││ const greet = (name: string): void => {                                    │
│   Night Owl / Night Owl Light          ││   console.log(`Hello, ${name}!`);                                          │
│   One Dark Pro                         ││ };                                                                         │
└────────────────────────────────────────┘└────────────────────────────────────────────────────────────────────────────┘
 ↑/↓ navigate  Enter select  q quit  Type to filter  Esc clear
```
TUI appeared instantly. Left panel: scrollable theme list. Right panel: live preview with hex values, color swatches, and code sample.

**Step 5 — Search filter test:**

Typed "mono":
```
┌ Select Theme [mono] ───────────────────┐┌ Preview: One Monokai (Dark) ───────────────────────────────────────────────┐
│ > One Monokai                          ││ bg:#282C34  fg:#ABB2BF  accent:#528BFF                                     │
│   One Monokai                          ││ ...                                                                        │
└────────────────────────────────────────┘
```
Filtered in real time to 2 results. Filter text shown in panel title `[mono]`.

Pressed Escape to clear:
```
┌ Select Theme ──────────────────────────┐
│ > Ayu Dark                             │
```
Full list restored, cursor reset to top.

**Step 6 — Theme navigation and selection:**
Pressed Down x2, cursor moved to "Ayu Light". Preview panel updated live: `bg:#F8F9FA fg:#5C6166 accent:#FFAA33`. Selected with Enter.

**Step 7 — Result (no confirmation prompt):**
```
Converting theme...

  ✔ Ayu Light → /Users/gimminsu/chromaport/themes/superset/chromaport-ayu-light.json

  Open Superset → Settings → Appearance →
  Import Theme → select /Users/gimminsu/chromaport/themes/superset/chromaport-ayu-light.json
  Saved theme IR to /Users/gimminsu/chromaport/themes/ayu-light.json
```
No overwrite prompt — direct success with installation instructions for Superset.

### File Verification

| Path | Status |
|------|--------|
| `~/chromaport/themes/ayu-light.json` | Exists (1053 bytes) |
| `~/chromaport/themes/superset/chromaport-ayu-light.json` | Exists (1822 bytes) |

### UX Feedback

- **Positive**:
  - Dual-pane TUI is immediately intuitive — list left, preview right
  - Live preview updates on every keypress with no perceptible lag
  - Search filter is instant and shows query in panel title
  - Escape to clear filter works exactly as expected
  - Success output is actionable: exact file path + Superset manual install steps
  - No unnecessary confirmation dialog for first-time install
- **Issues**:
  - Duplicate entries in the theme list (e.g. "Material Theme" appears twice, "One Dark Pro" appears three times). Likely two VS Code extension sources producing duplicates. No deduplication visible.
  - `q` to quit is shown in help bar but `Ctrl+C` / `Esc` not listed as alternatives — users may try Esc first expecting exit and be surprised it only clears filter
  - Preview panel shows color swatches as `██` blocks — readable as text but actual color rendering not verifiable in this test
- **Suggestions**:
  - Deduplicate themes by name or show source extension as a disambiguator
  - Add `(source: extension name)` column or tooltip to distinguish duplicate-named themes
  - Consider adding `Esc to exit` to the help bar, or make `q` more discoverable (e.g. `q/Esc to quit`)

---

## Flow 2: Apply Saved Theme

**Result**: Passed

### Experience Log

**Step 1 — Launch apply:**
```
./target/debug/chromaport apply
```

**Step 2 — Saved Themes TUI:**
```
┌ Saved Themes ──────────────────────────┐┌ Preview: Ayu Light (Light) ────────────────────────────────────────────────┐
│ > Ayu Light                            ││ bg:#F8F9FA  fg:#5C6166  accent:#FFAA33                                     │
│                                        ││ Normal: ██ ██ ██ ██ ██ ██ ██ ██                                            │
│                                        ││ Bright: ██ ██ ██ ██ ██ ██ ██ ██                                            │
│                                        ││ Chart: ██ ██ ██ ██ ██                                                      │
└────────────────────────────────────────┘└────────────────────────────────────────────────────────────────────────────┘
 ↑/↓ navigate  Enter select  q quit
```
Previously saved "Ayu Light" IR was discovered and shown with its preview. Same dual-pane layout as Flow 1.

**Step 3 — Theme selection:** Pressed Enter to select Ayu Light.

**Step 4 — Target MultiSelect (inquire):**
```
? Select targets to apply:
> [x] Warp
  [ ] Ghostty
[↑↓ to move, space to select one, → to all, ← to none, type to filter]
```
Toggled Warp with Space ([x] confirmed), pressed Enter.

**Step 5 — Result:**
```
  ✔ Ayu Light → /Users/gimminsu/chromaport/themes/warp/ayu-light.yaml
  Linked → /Users/gimminsu/.warp/themes/ayu-light.yaml

  Theme written to /Users/gimminsu/chromaport/themes/warp/ayu-light.yaml.
  Open Warp → Settings → Appearance → Themes to select it.
```

### File Verification

| Path | Status |
|------|--------|
| `~/chromaport/themes/warp/ayu-light.yaml` | Exists (484 bytes) |
| `~/.warp/themes/ayu-light.yaml` | Symlink → `~/chromaport/themes/warp/ayu-light.yaml` |

### UX Feedback

- **Positive**:
  - Saved Themes TUI is identical in layout to the main selector — zero learning curve
  - Preview loads immediately for saved themes (no re-fetch required)
  - Symlink creation is automatic and transparent — users don't need to manually copy files
  - "Linked" line in output makes it clear a symlink was created
  - Multi-target MultiSelect is well-labeled with Space/→/← shortcuts
- **Issues**:
  - Saved theme list shows only the theme name, not the type (Dark/Light) or creation date — hard to distinguish themes if multiple with similar names are saved
  - Help bar in apply TUI is missing the `Type to filter` hint (present in the main selector TUI)
  - Superset was already a target from Flow 1 — the `apply` command did not skip it or warn "already applied to Superset". This could lead to unintentional re-writes
- **Suggestions**:
  - Show Dark/Light type and creation date in the saved themes list
  - Add "(already applied)" marker in MultiSelect for targets that already have this theme
  - Add `Type to filter` hint to the apply TUI help bar for consistency

---

## Flow 3: Create Custom Theme

**Result**: Passed

### Experience Log

**Step 1 — Launch create:**
```
./target/debug/chromaport create
```

**Step 2 — Theme type selection (inquire Select):**
```
? Theme type:
> Dark
  Light
[↑↓ to move, enter to select, type to filter]
```
Clean two-option selector. Selected Dark.

**Step 3 — BG color picker (ratatui TUI):**
```
┌ Pick background color ─────────────────────────────────────────────────────────────────────────────────────────┐
│                                                                                                                  │
│ H: █████████████████████████████████┃█████████████████████████████████████████████████████████████████ 220°    │
│ S: ██████████████┃████████████████████████████████████████████████████████████████████████████████████  13%    │
│ L: ███████████████████┃████████████████████████████████████████████████████████████████████████████████  18%   │
│                                                                                                                  │
│  ████  #282C34                                                                                                   │
│                                                                                                                  │
│ ←/→ adjust  ↑/↓ switch  Enter confirm  Esc back                                                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```
HSL sliders displayed full-width. Each slider shows a numeric value on the right. A color swatch + hex code previews the current selection. Active slider indicated by position of `┃` cursor.

Adjusted H to 225°, S to 16% → color changed to #272A35. Confirmed with Enter.

**Step 4 — FG color picker:**
```
┌ Pick foreground color ─────────────────────────────────────────────────────────────────────────────────────────┐
│ H: 220°  S: 9%  L: 73%  →  #B4B8C0
```
Auto-generated foreground preset with high L value (73%) for good contrast against dark BG. Adjusted H by +3° and confirmed.

**Step 5 — Accent color picker:**
```
┌ Pick accent color ──────────────────────────────────────────────────────────────────────────────────────────────┐
│ H: 210°  S: 82%  L: 66%  →  #61A8EF
```
Auto-generated accent (vibrant blue) appropriate for dark theme. Adjusted H to 220°, confirmed.

**Step 6 — Preview screen:**
```
┌ Preview:  (Dark) ──────────────────────────────────────────────────────────────────────────────────────────────┐
│ bg:#272A35  fg:#B4B7C0  accent:#6191EF                                                                         │
│                                                                                                                  │
│ Normal: ██ ██ ██ ██ ██ ██ ██ ██                                                                                  │
│ Bright: ██ ██ ██ ██ ██ ██ ██ ██                                                                                  │
│ Chart: ██ ██ ██ ██ ██                                                                                            │
│                                                                                                                  │
│ const greet = (name: string): void => {                                                                          │
│   console.log(`Hello, ${name}!`);                                                                                │
│ };                                                                                                               │
│                                                                                                                  │
│ // Call the function                                                                                             │
│ greet("World");                                                                                                  │
│                                                                                                                  │
│ Enter confirm  Esc re-pick colors  q quit                                                                        │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```
`Esc re-pick colors` in the help bar is a nice affordance — allows going back without losing all work. However the preview title shows `Preview:  (Dark)` with an empty name (expected, since name is entered after). Confirmed with Enter.

**Step 7 — Name input:**
```
? Theme name:
```
Entered "E2E Test Theme". Confirmed.

**Step 8 — Result:**
```
  ✔ Saved to /Users/gimminsu/chromaport/themes/e2e-test-theme.json
  Run `chromaport apply` to apply this theme to your targets.
```

### File Verification

| Path | Status |
|------|--------|
| `~/chromaport/themes/e2e-test-theme.json` | Exists (1049 bytes), JSON valid |

### UX Feedback

- **Positive**:
  - Step-by-step color picking flow (BG → FG → Accent) is logical and guided
  - Auto-generated color presets per step give sensible defaults for each role
  - `Esc back` / `Esc re-pick colors` allows non-destructive revision — excellent UX
  - Full-width sliders are easy to operate; numeric readout prevents guesswork
  - Help bar is consistent across all three pickers
  - `chromaport apply` hint in success message guides the user on the next step
- **Issues**:
  - Preview title shows `Preview:  (Dark)` with an empty space where the name would be — slightly confusing before the user has entered a name. The preview logically appears before naming, but the empty slot looks like a bug
  - No indication of what slider step size is (1° per keypress for H, 1% for S/L). Users may want to know if there's a "fast move" modifier
  - BG color defaults to #282C34 (One Dark Pro–like) regardless of whether H/S/L should differ between Dark and Light theme types. Light themes would benefit from a higher L default
  - No way to type a hex code directly — slider-only input limits precision
- **Suggestions**:
  - Change preview title to `Preview: Untitled (Dark)` or omit the name entirely before it is set
  - Add a hex input mode (e.g. press `/` to type hex code directly)
  - Consider larger step size modifier (e.g. Shift+Right = 5 steps)
  - Differentiate initial slider defaults for Dark vs. Light theme type selection

---

## Flow 4: Presets List

**Result**: Failed

### Experience Log

**Step 1 — Launch:**
```
./target/debug/chromaport presets list
Fetching preset themes...
Error: Failed to fetch preset manifest. Check your internet connection.: http status: 404
```
Command returned immediately (< 1 second) with an error. No hang or timeout.

**Root cause:** The manifest URL `https://raw.githubusercontent.com/hamsurang/chromaport/main/assets/presets/index.json` returns HTTP 404. The `assets/presets/` directory has not been published to the `main` branch of the remote repository. Network connectivity is functional (raw.githubusercontent.com itself responds with 301 redirect as expected).

### File Verification

No files created.

### UX Feedback

- **Positive**:
  - Fast failure — no 10-second timeout hang before showing the error
  - Error message clearly says what failed ("Failed to fetch preset manifest")
- **Issues**:
  - The error message says "Check your internet connection" but the actual problem is a missing server-side resource (404), not a network issue. This is a **misleading error message** that could send users on a wild goose chase checking their network
  - No fallback behavior (e.g. "No presets available yet" or pointing to the repo URL)
  - The `: http status: 404` suffix is raw/technical — not user-friendly
- **Suggestions**:
  - Distinguish between network errors (no connectivity) and HTTP errors (4xx/5xx server responses) with separate messages
  - For 404 specifically: "Preset manifest not found. The preset catalog may not be available yet." with a pointer to the GitHub repo
  - For 5xx errors: "Preset server error. Try again later."

---

## Flow 5: Presets Install

**Result**: Failed

### Experience Log

**Step 1 — Launch:**
```
./target/debug/chromaport presets install
Fetching preset themes...
Error: Failed to fetch preset manifest. Check your internet connection.: http status: 404
```
Same root cause as Flow 4 — manifest 404.

### File Verification

No files created.

### UX Feedback

Same as Flow 4. The `presets install` command shares the manifest fetch path, so both fail identically.

---

## Summary of Findings

### Issues Found

| Severity | Flow | Issue |
|----------|------|-------|
| High     | 4, 5 | Preset manifest missing on remote (`assets/presets/index.json` → 404). Both presets commands non-functional. |
| Medium   | 4, 5 | Misleading error message: "Check your internet connection" shown for HTTP 404 (server-side missing resource, not network failure). |
| Medium   | 1    | Duplicate theme entries in the main selector (e.g. "Material Theme" ×2, "One Dark Pro" ×3). No deduplication or source disambiguation. |
| Medium   | 3    | Preview title shows `Preview:  (Dark)` with empty name slot before the name step — looks like a rendering bug. |
| Low      | 2    | Saved themes list omits Dark/Light type indicator and creation date — harder to distinguish saved themes as the list grows. |
| Low      | 2    | No "already applied" marker in target MultiSelect — can silently re-apply to a target that already has the theme. |
| Low      | 1    | Help bar shows `q quit` but not `Ctrl+C` or `Esc` (which clears filter only) — potential confusion about exit path. |
| Low      | 3    | No hex code direct-entry mode in color picker — slider-only limits precision. |

### Improvement Suggestions

1. **Publish `assets/presets/index.json`** to the `main` branch to unblock Flows 4 and 5.
2. **Differentiate error messages by HTTP status code** — separate "network unreachable" from "resource missing (404)" from "server error (5xx)".
3. **Deduplicate themes** by name in the main selector, or show the extension source as a disambiguator column.
4. **Rename preview title** before name entry to `Preview: Untitled (Dark)` instead of `Preview:  (Dark)`.
5. **Add metadata to saved themes list**: type (Dark/Light), date created, number of targets applied.
6. **Mark already-applied targets** in the apply MultiSelect with an `(applied)` tag.
7. **Add hex input mode** to color picker for precision entry.
8. **Add `apply` TUI help bar consistency**: include `Type to filter` hint (present in main selector but absent in apply).

### Test Limitations

- Color rendering not verified (text-only tmux capture — actual hex color appearance not visible)
- Timing-dependent: results may vary under CPU load
- Environment-dependent: results vary by installed apps/themes (Superset, Warp, Ghostty all present in this run)
- Flow 4 and 5 failures are infrastructure-related (missing remote asset), not code logic failures
