---
date: 2026-03-22
topic: wezterm-support
---

# WezTerm Terminal Theme Support

## Problem Frame

chromaport 사용자 중 WezTerm 터미널을 사용하는 사람들이 에디터 테마를 WezTerm으로 변환할 수 없다. WezTerm은 TOML 기반 color scheme을 지원하는 인기 터미널 에뮬레이터로, chromaport의 기존 output target 패턴(Ghostty, Warp 등)과 자연스럽게 호환된다.

## Requirements

- R1. ThemeIR에서 WezTerm TOML color scheme 파일을 생성할 수 있어야 한다
- R2. 생성된 TOML 파일은 `~/.config/wezterm/colors/` 디렉토리에 저장되어야 한다
- R3. 핵심 색상만 포함한다: foreground, background, cursor_bg, cursor_fg, cursor_border, selection_bg, selection_fg, ansi(8색), brights(8색)
- R4. `~/.config/wezterm` 또는 `~/.wezterm.lua` 존재 여부로 WezTerm 설치를 감지해야 한다
- R5. PostWriteAction은 ModifyConfig — `wezterm.lua`의 `color_scheme` 값을 자동 수정 (diff 표시 + 백업 + 사용자 확인, Ghostty 패턴과 동일)
- R6. 테마 파일명은 원본 이름을 유지한다 (예: `One Dark Pro.toml` → `config.color_scheme = 'One Dark Pro'`)
- R7. CLI의 Target enum에 `WezTerm` 옵션을 추가해야 한다

## Success Criteria

- WezTerm 사용자가 VS Code/Cursor/iTerm2/OpenCode 테마를 WezTerm color scheme으로 변환하고 바로 적용할 수 있다
- 기존 타겟(Ghostty, Warp 등)과 동일한 UX 패턴을 따른다
- 생성된 TOML 파일이 WezTerm에서 정상적으로 로드된다

## Scope Boundaries

- Output 전용 (WezTerm → 다른 포맷 변환은 포함하지 않음)
- tab_bar, scrollbar_thumb, split 등 확장 UI 색상은 포함하지 않음 (YAGNI)
- `[metadata]` 섹션은 포함하지 않음 (WezTerm은 파일명에서 scheme 이름을 추론)
- Windows 경로 지원은 범위 밖 (기존 타겟과 동일하게 macOS/Linux만)

## Key Decisions

- **핵심 색상만 포함**: tab_bar 등 확장 필드를 제외 — ThemeIR에서 정확하게 매핑 가능한 범위만 포함 (Ghostty 패턴과 동일, YAGNI)
- **ModifyConfig 방식**: wezterm.lua의 color_scheme을 자동 수정 — Ghostty의 config 수정 패턴을 그대로 따름 (diff + backup + 확인)
- **config 디렉토리 감지**: `~/.config/wezterm` 또는 `~/.wezterm.lua` 존재 여부로 감지 — WezTerm은 두 가지 config 위치를 지원하므로 둘 다 확인
- **원본 이름 유지**: WezTerm은 TOML 파일명(확장자 제외)을 scheme 이름으로 사용하므로 원본 테마 이름을 그대로 사용

## IR → WezTerm 색상 매핑

| WezTerm 필드 | ThemeIR 필드 |
|---|---|
| `background` | `terminal.background` |
| `foreground` | `terminal.foreground` |
| `cursor_bg` | `terminal.cursor` |
| `cursor_fg` | `background` (UI) |
| `cursor_border` | `terminal.cursor` |
| `selection_bg` | `selection_bg` |
| `selection_fg` | `foreground` (UI) |
| `ansi[0..7]` | `terminal.normal[0..7]` |
| `brights[0..7]` | `terminal.bright[0..7]` |

## WezTerm 테마 출력 예시

```toml
[colors]
foreground = "#abb2bf"
background = "#282c34"
cursor_bg = "#528bff"
cursor_fg = "#282c34"
cursor_border = "#528bff"
selection_bg = "#3e4451"
selection_fg = "#abb2bf"
ansi = ["#282c34", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf"]
brights = ["#545862", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf"]
```

## Outstanding Questions

### Deferred to Planning

- [Affects R5][Technical] wezterm.lua는 Lua 파일이므로 config 수정 시 Lua 문법을 파싱해야 하는지, 아니면 regex/문자열 치환으로 충분한지
- [Affects R4][Needs research] macOS에서 WezTerm이 `~/Library/Application Support/` 경로도 사용하는지 확인 필요
- [Affects R2][Technical] `~/.config/wezterm/colors/` 디렉토리가 없을 때 자동 생성 여부 (기존 타겟 패턴 따르면 됨)

## Next Steps

→ `/ce:plan` for structured implementation planning
