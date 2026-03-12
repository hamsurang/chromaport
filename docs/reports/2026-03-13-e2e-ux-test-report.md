# chromaport E2E UX Test Report
Date: 2026-03-13
Binary: ./target/debug/chromaport (git: 558cc4e)
Environment: macOS 26.2, tmux 3.6a

## Executive Summary
- Total flows: 5
- Passed: 3
- Failed: 2
- Skipped: 0

## Environment
| Dependency   | Status | Detail |
|--------------|--------|--------|
| tmux         | OK     | 3.6a |
| cargo build  | OK     | chromaport v0.7.0, 3.59s |
| VS Code      | OK     | ~/.vscode/extensions present |
| Cursor       | OK     | ~/.cursor/extensions present |
| Superset     | OK     | ~/.superset present |
| Warp         | OK     | ~/.warp present |
| Ghostty      | OK     | ~/.config/ghostty present |
| Network      | OK     | raw.githubusercontent.com → HTTP 301 |
| Preset catalog | FAIL | assets/presets/index.json not on remote main branch |

---

## Flow 1: Default Theme Selection
**Result**: Passed

### Experience Log

**Step 1 — Launch**
```
? Select editor:
> VS Code
  Cursor
[↑↓ to move, enter to select, type to filter]
```
Two editors detected (VS Code, Cursor). Inquire Select appeared immediately with no delay.

**Step 2 — Editor selected → Target selection**
```
> Select editor: VS Code
? Select target app:
> Superset
  Warp
  Ghostty
```
Three target apps detected. Clean sequential prompt chaining. No confusion between steps.

**Step 3 — TUI theme list**
```
┌ Select Theme ──────────────────────────┐┌ Preview: Ayu Dark (Dark) ──────────────────────────────────────────────────┐
│ > Ayu Dark                             ││ bg:#0B0E14  fg:#BFBDB6  accent:#E6B450                                     │
│   Ayu Dark Bordered                    ││                                                                            │
│   Ayu Light (saved)                    ││ Normal: ██ ██ ██ ██ ██ ██ ██ ██                                            │
│   Ayu Light Bordered                   ││ Bright: ██ ██ ██ ██ ██ ██ ██ ██                                            │
│   Ayu Mirage                           ││                                                                            │
│   ...                                  ││ Chart: ██ ██ ██ ██ ██                                                      │
│   One Dark Pro (One Dark Pro)          ││                                                                            │
└────────────────────────────────────────┘│ const greet = (name: string): void => {                                    │
 ↑/↓ navigate  Enter select  q quit  Type to filter  Esc clear
```
Split-pane TUI loaded instantly. Preview panel shows bg/fg/accent hex codes, color swatches (Normal x8, Bright x8, Chart x5), and a TypeScript code snippet. "Ayu Light (saved)" marker correctly identifies a previously saved theme.

**Step 4 — Search filter "mono"**
```
┌ Select Theme [mono] ───────────────────┐┌ Preview: One Monokai (Dark) ──────────────────────────────────────────────┐
│ > One Monokai (One Monokai Theme)      ││ bg:#282C34  fg:#ABB2BF  accent:#528BFF                                     │
│   One Monokai (One Monokai Theme)      ││ ...                                                                        │
```
Filter applied instantly on keystroke. Title bar updated to `[mono]`. 2 matching results shown. Preview auto-updated to first filtered result.

**Step 5 — Escape to clear filter**
Full list restored immediately. Cursor returned to first item (Ayu Dark). Filter title cleared.

**Step 6 — Theme selection (3rd item: Ayu Light)**
Navigation: Down x2, Enter. Overwrite prompt appeared because file already existed:
```
? /Users/gimminsu/chromaport/themes/superset/chromaport-ayu-light.json already exists. Overwrite? (y/N)
```
Confirmed with `y`. Result:
```
  ✔ Ayu Light → /Users/gimminsu/chromaport/themes/superset/chromaport-ayu-light.json
  Open Superset → Settings → Appearance →
  Import Theme → select /Users/gimminsu/chromaport/themes/superset/chromaport-ayu-light.json
  Saved theme IR to /Users/gimminsu/chromaport/themes/ayu-light.json
```

### File Verification
| Path | Status |
|------|--------|
| ~/chromaport/themes/ayu-light.json | Exists (1083 bytes) |
| ~/chromaport/themes/superset/chromaport-ayu-light.json | Exists (1822 bytes) |

### UX Feedback
- **Positive**:
  - TUI loads instantly with no perceptible delay
  - Split-pane layout is very effective — theme list + live preview in one view
  - Search filter is real-time and the `[filter]` title indicator is clear
  - `(saved)` marker in the theme list helps users identify known themes
  - Overwrite prompt is safe (defaults to No)
  - Post-apply instructions (Settings → Appearance → Import Theme) are actionable and specific
- **Issues**:
  - Theme list entries are truncated at the right edge (e.g., `Material Theme (Material Theme — Free` cut off). Extension source names are long, making them hard to distinguish at a glance
  - Duplicate theme names appear (e.g., two "One Monokai (One Monokai Theme)" entries with no visual differentiation). Users cannot tell which extension version they are selecting
  - Preview panel has a large empty area below the code snippet — wasted vertical space
- **Suggestions**:
  - Truncate extension names in parentheses more aggressively, or show them in a dimmed secondary style
  - Add a deduplication indicator or source badge to distinguish same-named themes from different extension versions
  - Fill preview panel empty space with more information (e.g., terminal color palette, theme metadata)

---

## Flow 2: Apply Saved Theme
**Result**: Passed

### Experience Log

**Step 1 — Launch apply**
```
┌ Saved Themes ──────────────────────────┐┌ Preview: Ayu Light (Light) ────────────────────────────────────────────────┐
│ > Ayu Light [Light] 2026-03-12         ││ bg:#F8F9FA  fg:#5C6166  accent:#FFAA33                                     │
│   E2E Test Theme [Dark]                ││ ...                                                                        │
└────────────────────────────────────────┘
 ↑/↓ navigate  Enter select  q quit  Type to filter  Esc clear
```
Saved themes TUI shows both saved IRs with type badge `[Light]`/`[Dark]` and creation date. Preview panel mirrors the theme select TUI layout.

**Step 2 — Theme selected → Target MultiSelect**
```
? Select targets to apply:
> [ ] Superset (applied)
  [ ] Warp (applied)
  [x] Ghostty
[↑↓ to move, space to select one, → to all, ← to none, type to filter]
```
`(applied)` markers correctly shown for Superset and Warp (applied in Flow 1). Ghostty pre-checked as it had not yet received this theme. This is smart state-awareness.

**Step 3 — Ghostty config diff and confirmation**
```
  Changes to /Users/gimminsu/Library/Application Support/com.mitchellh.ghostty/config:
     keybind = cmd+\=new_split:right
     keybind = cmd+shift+\=new_split:down
    -theme = Ayu Dark
    +theme = Ayu Light
? Apply to Ghostty config? (y/N)
```
Config diff is clear and specific. Shows only the relevant changed line, with surrounding context. Confirmed with `y`.

**Step 4 — Result**
```
  ✔ Backed up → /Users/gimminsu/Library/Application Support/com.mitchellh.ghostty/config.bak.1773332367
  ✔ Updated config
    Reload Ghostty config (Cmd+Shift+,) to apply.
```
Backup created before modifying live config. Reload instruction (Cmd+Shift+,) is immediately actionable.

### File Verification
| Path | Status |
|------|--------|
| ~/chromaport/themes/ghostty/Ayu Light | Exists (475 bytes) |
| ~/.config/ghostty/themes/Ayu Light | Symlink → ~/chromaport/themes/ghostty/Ayu Light |
| Ghostty config backup | Exists (.bak.1773332367) |

### UX Feedback
- **Positive**:
  - `(applied)` markers in MultiSelect are an excellent UX detail — prevents redundant re-applying
  - Config diff before destructive write is excellent safety UX
  - Automatic backup before config modification is reassuring
  - Ghostty reload shortcut hint is very practical
  - Date and type badge in saved themes TUI provide enough context to identify themes
- **Issues**:
  - The MultiSelect pre-checks Ghostty but not Superset/Warp. The logic seems to pre-check targets that are NOT already applied, which is correct, but could be confusing — user must understand what "(applied)" means to know why some are unchecked
  - No explanation of what selecting "(applied)" targets will do (re-apply? skip? overwrite?)
- **Suggestions**:
  - Add a brief hint like "Already applied targets will be re-applied" or "(applied) = previously applied, re-select to update"
  - Consider showing the current applied theme name next to `(applied)` to show which version is installed

---

## Flow 3: Create Custom Theme
**Result**: Passed

### Experience Log

**Step 1 — Theme type selection**
```
? Theme type:
> Dark
  Light
[↑↓ to move, enter to select, type to filter]
```
Simple 2-option select. Clear and unambiguous.

**Step 2 — Background color picker (HSL)**
```
┌ Pick background color ────────────────────────────────────────────────────────────────────────────────────────────────┐
│ H: █████████████████████████████████████████████████████████████████┃█████████████████████████████████████████ 220°  │
│ S: ██████████████┃████████████████████████████████████████████████████████████████████████████████████████████  13%  │
│ L: ███████████████████┃███████████████████████████████████████████████████████████████████████████████████████  18%  │
│                                                                                                                      │
│  ████  #282C34                                                                                                       │
│                                                                                                                      │
│ ←/→ adjust  ↑/↓ switch  # hex input  Enter confirm  Esc back                                                         │
└───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```
Full-width slider UI. Each slider occupies the entire terminal width for fine control. Hex preview updates live. Help bar clearly documents all interactions including `# hex input`.

**Step 3 — BG adjustments (H: 220→225°, S: 13→16%)**
After Right x5 on H, Down, Right x3 on S — hex updated from `#282C34` to `#272A35`. Responsive, no input lag.

**Step 4 — FG color picker transition**
Transitioned to "Pick foreground color" after Enter. Sliders pre-set to sensible defaults for a dark theme foreground (H:220°, S:9%, L:73%, #B4B8C0). Applied Right x3 adjustment.

**Step 5 — Accent color picker transition**
Transitioned to "Pick accent color". Pre-set to saturated blue (H:210°, S:82%, L:66%, #61A8EF). Applied Right x10. Final hex: `#6191EF`.

**Step 6 — Preview confirmation screen**
```
┌ Preview: Untitled (Dark) ──────────────────────────────────────────────────────────────────────────────────────────────┐
│ bg:#272A35  fg:#B4B7C0  accent:#6191EF                                                                                 │
│                                                                                                                        │
│ Normal: ██ ██ ██ ██ ██ ██ ██ ██                                                                                        │
│ Bright: ██ ██ ██ ██ ██ ██ ██ ██                                                                                        │
│ Chart: ██ ██ ██ ██ ██                                                                                                  │
│                                                                                                                        │
│ const greet = (name: string): void => {                                                                                │
│   console.log(`Hello, ${name}!`);                                                                                      │
│ };                                                                                                                     │
│ // Call the function                                                                                                   │
│ greet("World");                                                                                                        │
│                                                                                                                        │
│ Enter confirm  Esc re-pick colors  q quit                                                                              │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```
Preview shows "Untitled (Dark)" title (correct for unnamed theme in-progress). Help bar correctly shows "Esc re-pick colors" — allowing back navigation.

**Step 7 — Name input and save**
```
? Theme name:
> Theme name: E2E Test Theme
  ✔ Saved to /Users/gimminsu/chromaport/themes/e2e-test-theme.json
  Run `chromaport apply` to apply this theme to your targets.
```

### File Verification
| Path | Status |
|------|--------|
| ~/chromaport/themes/e2e-test-theme.json | Exists (1079 bytes), valid JSON |
| JSON keys | id, name, theme_type, background, foreground, accent, cursor, selection_bg, border, sidebar_bg, sidebar_fg, input_bg, muted_fg, chart_colors, terminal, created_at |

### UX Feedback
- **Positive**:
  - Full-width HSL sliders are excellent for fine-grained color control
  - Live hex preview (`████ #RRGGBB`) is very effective text-only color feedback
  - Step-by-step BG → FG → Accent flow is natural and guided
  - "Esc re-pick colors" escape hatch on preview is very UX-considerate
  - "Untitled" placeholder title in preview is correct (previously verified this was a fix)
  - `created_at` field in saved JSON enables date display in apply TUI
  - Post-save hint "Run `chromaport apply`" provides clear next action
- **Issues**:
  - After pressing Escape in the color picker, it's unclear whether Esc goes back one step (BG → type selection?) or exits entirely. The help bar says "Esc back" but "back to what?" is ambiguous
  - No color preview of the actual rendered colors — only block characters `██` which render as solid colored squares only in a true-color terminal. In a 256-color or non-color terminal, the preview would lose most of its value
  - During color picking, there's no comparison to "Dark theme typical range" — users unfamiliar with HSL may pick an unreadable foreground (e.g., L too low)
- **Suggestions**:
  - Make Esc behavior explicit in help bar: "Esc = back to [BG/FG/Accent] picker"
  - Add contrast ratio indicator (foreground vs background) in the preview for accessibility guidance
  - Consider showing a brief HSL guide ("L < 30% = dark, L > 70% = light") when cursor is on L slider

---

## Flow 4: Presets List
**Result**: Failed

### Experience Log

**Command**: `./target/debug/chromaport presets list`

**Output**:
```
Fetching preset themes...
Error: preset catalog not found. It may not be available yet.
```

**Root Cause**: The MANIFEST_URL (`https://raw.githubusercontent.com/hamsurang/chromaport/main/assets/presets/index.json`) returns HTTP 404. The `assets/presets/` directory exists in the local `import-apply` branch (commit `aa17059 feat: add presets subcommand for bundled theme installation`) but has not been merged to the remote `main` branch yet.

**Local assets verified**:
```
assets/presets/
  index.json
  ayu-dark.json, catppuccin-mocha.json, dracula.json, github-dark.json,
  gruvbox-dark.json, material-theme.json, nord.json, one-monokai.json,
  solarized-dark.json, solarized-light.json, tokyo-night.json
```
11 preset themes are ready locally but unreachable from the binary.

### File Verification
| Path | Status |
|------|--------|
| assets/presets/index.json (local) | Exists |
| https://raw.githubusercontent.com/.../main/assets/presets/index.json | HTTP 404 |

### UX Feedback
- **Positive**:
  - Error message is user-friendly: "preset catalog not found. It may not be available yet." — does not expose raw HTTP errors
- **Issues**:
  - Feature is non-functional in current state — requires merging `import-apply` to `main` and pushing `assets/` to remote
  - No fallback to local bundled assets even though they exist in the binary's working tree
- **Suggestions**:
  - Consider bundling preset manifests directly into the binary (using `include_str!` or a build script) so the feature works without network access or remote file availability
  - Alternatively, ship `assets/presets/` as part of the release artifact and reference them from a local path as fallback

---

## Flow 5: Presets Install
**Result**: Failed

### Experience Log

**Root Cause**: Same as Flow 4. `fetch_manifest()` calls the same MANIFEST_URL which returns 404. The MultiSelect UI for preset selection is never reached.

**Output**:
```
Fetching preset themes...
Error: preset catalog not found. It may not be available yet.
```

### File Verification
| Path | Status |
|------|--------|
| ~/chromaport/themes/ new preset files | None created |

### UX Feedback
- **Positive**:
  - Error message consistent with Flow 4
- **Issues**:
  - Cannot verify MultiSelect UI, download progress, or installation UX
  - The `(installed)` marker logic cannot be tested until the catalog is reachable
- **Suggestions**:
  - Same as Flow 4: bundle assets or add local fallback
  - Add a `--local` flag or `presets install --from-path` for offline/dev use

---

## Summary of Findings

### Issues Found

| Severity | Flow | Issue |
|----------|------|-------|
| High | 4, 5 | Preset catalog HTTP 404 — assets/presets/ not on remote main branch; Flows 4 and 5 completely non-functional |
| Medium | 1 | Theme name truncation in list panel — extension source names overflow the fixed-width column, making duplicates indistinguishable |
| Medium | 1 | Duplicate theme entries with no visual differentiation — two "One Monokai (One Monokai Theme)" entries appear identical |
| Medium | 2 | Apply MultiSelect "(applied)" marker has no explanatory hint — users may not understand what re-selecting an applied target will do |
| Low | 3 | Esc behavior in color picker is ambiguous — "Esc back" does not specify what step it returns to |
| Low | 3 | No contrast ratio / accessibility indicator in theme preview |
| Low | 1 | Preview panel has large unused vertical space below code snippet |

### Improvement Suggestions

1. **Merge `assets/presets/` to main**: The preset feature is blocked until `import-apply` is merged. This is the highest priority fix.
2. **Bundle preset manifests in binary**: Use `include_str!` macro or a build-time step to embed the preset catalog, enabling offline and pre-release usage.
3. **Theme list deduplication UX**: Show extension source in a dimmed/secondary style or add a distinct suffix to distinguish same-named themes from different VS Code extensions.
4. **Apply MultiSelect hint**: Add a one-line hint explaining what re-selecting `(applied)` targets does.
5. **Color picker Esc clarity**: Update help bar to "Esc = back to BG picker" (or FG/Accent as appropriate for the current step).
6. **Contrast ratio in preview**: Show a computed contrast ratio (WCAG AA/AAA) between bg and fg colors in the create preview screen.
7. **Preview panel density**: Fill unused vertical space in the theme preview panel with additional information such as full terminal palette or theme metadata.

### Test Limitations
- Color rendering not verified (text-only tmux capture — block characters `██` may or may not render as colored squares depending on terminal)
- Timing-dependent: results may vary under CPU load (all timing within expected range during this run)
- Environment-dependent: VS Code/Cursor extension count not verified; results vary by installed themes
- Flow 4 and 5 could not be fully exercised due to infrastructure gap (assets not on remote main)
- Esc back-navigation in color picker not tested end-to-end (only forward path exercised)
