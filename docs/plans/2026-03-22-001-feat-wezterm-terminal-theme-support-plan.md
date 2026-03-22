---
title: "feat: Add WezTerm terminal theme support"
type: feat
status: active
date: 2026-03-22
deepened: 2026-03-22
origin: docs/brainstorms/2026-03-22-wezterm-support-requirements.md
---

# feat: Add WezTerm Terminal Theme Support

## Enhancement Summary

**Deepened on:** 2026-03-22
**Research agents used:** Lua config patterns, architecture review, security audit, code simplicity, TOML format verification

### Key Improvements
1. **CRITICAL Security fix**: Lua code injection 방어를 위한 `lua_escape_string()` 추가
2. **Config 우선순위 수정**: WezTerm 공식 문서 기반 정확한 resolution order 반영
3. **Phase 통합**: 6 phases → 3 phases로 간소화 (simplicity review 반영)
4. **Lua 수정 전략 강화**: 주석 처리, 따옴표 스타일, `color_schemes` 구분 등 엣지 케이스 보강
5. **파일 권한**: `atomic_write` 후 0o644 권한 설정 명시

### New Considerations Discovered
- Theme name을 Lua 파일에 삽입할 때 code injection 벡터 존재 → 이스케이프 필수
- `color_schemes` (복수형), `color_scheme_dirs` 와의 구분 필요
- WezTerm은 macOS 전용 경로(`~/Library/Application Support/`) 미사용 확인됨
- regex crate 불필요 — Ghostty처럼 plain string operations (`starts_with`, `split_once`) 사용

---

## Overview

chromaport에 WezTerm 터미널을 새로운 output target으로 추가하여, 에디터 테마(VS Code, Cursor, OpenCode, iTerm2)를 WezTerm TOML color scheme으로 변환할 수 있게 한다. Ghostty 타겟 구현 패턴을 따르며, ModifyConfig를 통해 `wezterm.lua`의 `color_scheme` 설정을 자동 수정한다.

## Problem Statement / Motivation

WezTerm은 크로스 플랫폼 GPU-가속 터미널 에뮬레이터로, 활발한 사용자 커뮤니티를 가지고 있다. chromaport은 이미 Ghostty, Warp, iTerm2 등 6개 타겟을 지원하지만 WezTerm 사용자는 에디터 테마를 수동으로 변환해야 한다. 기존 Target 아키텍처가 잘 정립되어 있어 최소한의 변경으로 추가 가능하다. (see origin: docs/brainstorms/2026-03-22-wezterm-support-requirements.md)

## Proposed Solution

Ghostty 타겟 패턴을 그대로 따라 `src/target/wezterm.rs` 모듈을 추가한다:
- `format!()`을 사용한 TOML 생성 (toml crate 불필요 — 구조가 단순)
- `~/.config/wezterm/colors/` 디렉토리에 symlink
- `wezterm.lua`의 `color_scheme` 값을 line-based string matching으로 수정 (ModifyConfig)
- 핵심 색상만 포함 (YAGNI, see origin)

## Technical Considerations

### Lua Config 수정 전략

WezTerm config는 Lua 파일이므로 Ghostty의 plain-text 방식과 다르다. 하지만 full Lua parser는 과도하며, Ghostty처럼 plain string operations (`starts_with`, `split_once` — regex crate 불필요)로 충분하다.

**지원할 패턴 (실제 dotfile 조사 기반):**

1. **Config builder 패턴** (가장 일반적):
   ```lua
   config.color_scheme = "Gruvbox Dark"
   config.color_scheme = 'Gruvbox Dark'  -- single quotes도 지원
   ```

2. **Return table 패턴**:
   ```lua
   return {
     color_scheme = "Solarized Dark",  -- trailing comma 보존
   }
   ```

3. **패턴 미발견 시**: `return config` 직전에 `config.color_scheme = "X"` 삽입. `return config`도 없으면 Guide fallback.

**지원하지 않을 패턴 (Guide fallback):**
- 조건부 할당 (`if ... then config.color_scheme = "X"`)
- 함수 호출 (`config:set("color_scheme", "X")`)
- 변수 참조 (`color_scheme = some_var`)
- 복수 매칭 시 (같은 키가 2번 이상 나타남 — 조건부일 가능성)

**핵심 구현 규칙:**
- `--`로 시작하는 Lua 주석 라인은 반드시 건너뛰기
- `color_schemes` (복수형), `color_scheme_dirs`와 구분: `=` 기준으로 split 후 key 부분이 정확히 `color_scheme`인지 확인
- 값이 따옴표로 감싸진 문자열 리터럴인 경우만 매칭 (동적 값은 Guide fallback)
- 원본 따옴표 스타일(`"` 또는 `'`) 보존
- trailing comma 및 inline comment 보존

### Security: Lua Code Injection 방어

**CRITICAL**: Theme name은 사용자 입력(VS Code extension label 등)에서 유래하며, Lua 소스 코드에 삽입된다. 이스케이프 없이 삽입하면 code injection이 가능하다.

```rust
/// Lua string literal context에서 안전한 이스케이프
fn lua_escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\'', "\\'")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\0', "")
}
```

- `set_color_scheme_in_config` 호출 전에 theme name을 반드시 이스케이프
- 기존 값 교체 시에는 이스케이프된 name을 원본 따옴표 스타일로 감싸기
- 테스트 필수: `"; os.execute("rm -rf /"); --` 같은 payload가 안전한 출력을 생성하는지 검증

### Backup 파일명 이슈

현재 `handle_post_write_action` (src/target/mod.rs:164)에서 backup 파일명이 `config.bak.{ts}`로 하드코딩됨. `wezterm.lua`의 경우 원본 파일명과 확장자가 유실된다. OpenCode의 `tui.json`에도 동일 버그 존재.

**수정:** `{stem}.bak.{ts}.{ext}` 형식으로 변경. Phase 2에서 같은 파일(`mod.rs`) 수정 시 함께 적용.

```rust
// Before
let backup = config_path.with_file_name(format!("config.bak.{}", timestamp));

// After
let backup = {
    let stem = config_path.file_stem().unwrap_or_default().to_string_lossy();
    match config_path.extension() {
        Some(ext) => config_path.with_file_name(
            format!("{}.bak.{}.{}", stem, timestamp, ext.to_string_lossy())
        ),
        None => config_path.with_file_name(
            format!("{}.bak.{}", stem, timestamp)
        ),
    }
};
```

Ghostty의 config 파일명이 `config`(확장자 없음)이므로 `config.bak.{ts}` → 동일 결과, 하위 호환.

### Config 파일 위치 우선순위

WezTerm 공식 문서 기반 resolution order (first-match-wins, 병합 없음):

| 우선순위 | 경로 |
|---|---|
| 1 | `$WEZTERM_CONFIG_FILE` (환경변수) |
| 2 | `$XDG_CONFIG_HOME/wezterm/wezterm.lua` |
| 3 | `~/.config/wezterm/wezterm.lua` |
| 4 | `~/.wezterm.lua` (home fallback) |

> 참고: 기존 brainstorm에서는 `~/.wezterm.lua`가 우선이라고 했으나, WezTerm 공식 문서에 의하면 XDG 경로가 우선이고 `~/.wezterm.lua`는 fallback이다.

macOS 전용 경로(`~/Library/Application Support/`)는 사용하지 않음 (확인 완료).

`wezterm_config_path()` 함수 하나로 detection과 config 수정 모두에 사용하여 일관성 보장.

### 파일 권한

`atomic_write()` 기본 권한은 0o600. WezTerm TOML 파일은 비밀 정보가 아니므로 iTerm2/Obsidian 패턴과 동일하게 0o644로 override 필요:

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&theme_path, std::fs::Permissions::from_mode(0o644));
}
```

### IR → WezTerm 색상 매핑

| WezTerm 필드 | ThemeIR 필드 | 비고 |
|---|---|---|
| `background` | `terminal.background` | |
| `foreground` | `terminal.foreground` | |
| `cursor_bg` | `terminal.cursor` | |
| `cursor_fg` | `background` (UI) | Ghostty cursor-text 패턴 동일 |
| `cursor_border` | `terminal.cursor` | cursor_bg와 동일값 (관례) |
| `selection_bg` | `terminal.selection_bg ?? selection_bg` | terminal 우선, UI fallback |
| `selection_fg` | `foreground` (UI) | ThemeIR에 selection_fg 없음, Ghostty 패턴 동일 |
| `ansi[0..7]` | `terminal.normal[0..7]` | |
| `brights[0..7]` | `terminal.bright[0..7]` | |

### TOML 출력 포맷 (검증 완료)

iTerm2-Color-Schemes 및 Catppuccin 등 실제 WezTerm TOML 파일과 대조 검증 완료:
- `[colors]` 섹션 헤더 필수 ✓
- Hex 값 반드시 따옴표 감싸기 (`"#abb2bf"`) ✓
- 키는 snake_case (`cursor_bg`, `selection_fg`) ✓
- `ansi`/`brights`는 TOML 배열 ✓
- `[metadata]` 생략 가능 (filename에서 scheme name 추론) ✓
- 모든 필드 선택적이나, 위 9개 필드 + 16색이 실질적 최소 ✓

```toml
# Generated by chromaport
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

## Acceptance Criteria

- [ ] `chromaport -e vscode -t wezterm`으로 WezTerm TOML 파일 생성
- [ ] 생성된 TOML이 WezTerm에서 정상 로드 (`[colors]` 섹션, ansi/brights 배열)
- [ ] `~/.config/wezterm/colors/`에 symlink 생성
- [ ] `wezterm.lua`의 `color_scheme` 값 자동 수정 (diff + backup + 확인)
- [ ] Lua code injection 방어 (이스케이프 + 테스트)
- [ ] TOML 파일 권한 0o644
- [ ] `chromaport apply`에서 WezTerm 타겟 선택 가능
- [ ] `chromaport create`으로 생성한 커스텀 테마도 WezTerm 출력 가능
- [ ] backup 파일명이 원본 파일명/확장자 보존
- [ ] 기존 테스트 통과 + WezTerm 관련 테스트 추가

## Implementation Phases

### Phase 1: WezTerm 타겟 모듈 구현 + 단위 테스트

**새 파일: `src/target/wezterm.rs`**

구현할 함수 (Ghostty 패턴 참조 — 로직 복사, 공유 함수 추출 불필요):

1. **`wezterm_config_dir() -> Option<PathBuf>`** — XDG config 디렉토리 (`~/.config/wezterm`). XDG validation 필터 포함: `.filter(|s| !s.is_empty() && Path::new(s).is_absolute())`
2. **`wezterm_config_path() -> Option<PathBuf>`** — Lua config 파일 경로. 우선순위: `$WEZTERM_CONFIG_FILE` > `$XDG_CONFIG_HOME/wezterm/wezterm.lua` > `~/.config/wezterm/wezterm.lua` > `~/.wezterm.lua`
3. **`detect() -> bool`** — `wezterm_config_dir().exists()` 또는 `~/.wezterm.lua` 존재
4. **`write(ir: &ThemeIR) -> Result<PathBuf>`** — `chromaport_themes_dir("wezterm")`에 TOML 생성, `atomic_write` + 0o644 권한 설정
5. **`existing_theme_path(ir: &ThemeIR) -> Option<PathBuf>`** — 중앙 저장소에서 기존 파일 확인
6. **`link(ir: &ThemeIR, written_path: &Path) -> LinkResult`** — `~/.config/wezterm/colors/{name}.toml`로 symlink. `create_symlink`이 parent directory 자동 생성 처리
7. **`post_write_action(ir: &ThemeIR) -> PostWriteAction`** — ModifyConfig 반환. config 파일 없고 config dir 존재 시 `CreateConfig` (`~/.config/wezterm/wezterm.lua` 경로로 생성). config dir도 없으면 Guide
8. **`format_wezterm_theme(ir: &ThemeIR) -> String`** — TOML 출력 생성. `#RRGGBBAA` → `#RRGGBB` strip 포함
9. **`lua_escape_string(s: &str) -> String`** — Lua injection 방어용 이스케이프
10. **`set_color_scheme_in_config(content: &str, scheme_name: &str) -> Option<String>`** — Lua config 수정. `--` 주석 건너뛰기, `color_schemes`/`color_scheme_dirs` 구분, 따옴표 스타일 보존, trailing comma 보존. 복수 매칭 또는 비리터럴 값이면 `None` 반환 → Guide fallback
11. **`theme_filename(name: &str) -> String`** — Ghostty 로직 복사 (원본 이름 보존, filesystem 위험 문자 치환) + `.toml` 확장자 함수 내부에서 추가

**단위 테스트 (`#[cfg(test)] mod tests` — 같은 파일 내):**
- `format_wezterm_theme_correct_output` — TOML 출력 형식 검증
- `set_color_scheme_replaces_config_builder` — `config.color_scheme = "old"` → `config.color_scheme = "new"`
- `set_color_scheme_replaces_table_style` — `color_scheme = "old",` → `color_scheme = "new",` (trailing comma 보존)
- `set_color_scheme_handles_single_quotes` — `config.color_scheme = 'old'` → `config.color_scheme = 'new'`
- `set_color_scheme_skips_comments` — `-- config.color_scheme = "old"` 무시, 실제 라인만 수정
- `set_color_scheme_skips_color_schemes_plural` — `color_schemes = { ... }` 무시
- `set_color_scheme_returns_none_for_conditional` — `if ... then config.color_scheme` → None
- `set_color_scheme_returns_none_for_multiple_matches` — 같은 키 2회 이상 → None
- `set_color_scheme_inserts_before_return` — 기존 설정 없을 때 `return config` 전에 삽입
- `set_color_scheme_returns_none_when_no_return` — 삽입 위치 없으면 None (Guide)
- `lua_escape_string_prevents_injection` — `"; os.execute("rm -rf /"); --` → 안전한 출력
- `lua_escape_string_handles_quotes_and_backslash` — 특수 문자 이스케이프 검증
- `theme_filename_preserves_name` — 원본 이름 보존 + `.toml`
- `theme_filename_sanitizes_unsafe_chars` — `/ \ \0 : \n \r` 치환

### Phase 2: 등록, dispatch, backup 수정, 에러 메시지

**수정 파일:**

1. **`src/cli.rs:86-93`** — `Target` enum에 `Wezterm` 추가
2. **`src/cli.rs:6`** — about 문자열에 "WezTerm" 추가
3. **`src/target/mod.rs:1`** — `pub mod wezterm;` 추가
4. **`src/target/mod.rs:47-126`** — 7개 dispatch 함수 + `all()` 배열 크기 `[Target; 7]`:
   - `detect`: `Target::Wezterm => wezterm::detect()`
   - `write`: `Target::Wezterm => wezterm::write(ir)`
   - `existing_theme_path`: `Target::Wezterm => wezterm::existing_theme_path(ir)`
   - `link`: `Target::Wezterm => wezterm::link(ir, written_path)`
   - `post_write_action`: `Target::Wezterm => wezterm::post_write_action(ir)`
   - `display_name`: `Target::Wezterm => "WezTerm"`
   - `all()`: `[Target; 7]` + `Target::Wezterm`
5. **`src/target/mod.rs:164`** — backup 파일명 수정 (`{stem}.bak.{ts}.{ext}`)
6. **에러 메시지 업데이트** (타겟 목록 나열하는 모든 위치):
   - `src/main.rs:147` — 타겟 경로 에러
   - `src/main.rs:208` — OpenCode import 에러
   - `src/main.rs:265` — iTerm2 import 에러
   - `src/apply.rs:43` — apply 에러
   - `src/interactive.rs:61` — interactive 에러
7. **`Cargo.toml:5`** — description 필드에 WezTerm 추가
8. **`Cargo.toml:9`** — keywords 배열에 `"wezterm"` 추가

**통합 테스트 (tests/cli.rs):**
- `wezterm_target_accepted` — `--target wezterm` 옵션이 유효한 값으로 인식

**Backup 테스트 (src/target/mod.rs 내):**
- `backup_preserves_lua_extension` — `wezterm.lua` → `wezterm.bak.{ts}.lua`
- `backup_handles_no_extension` — `config` → `config.bak.{ts}`

### Phase 3: 문서 업데이트 + version bump

- `README.md` — 지원 타겟 목록에 WezTerm 추가
- `README.ko.md` — 동일
- `Cargo.toml` — version bump (minor: feat이므로)

## Dependencies & Risks

- **의존성 없음**: toml crate 불필요 (`format!()` 사용), regex crate 불필요 (plain string ops)
- **CRITICAL Risk: Lua code injection**: theme name이 Lua 소스에 삽입됨. **미티게이션**: `lua_escape_string()` + 테스트
- **Risk: Lua config 수정**: line-based matching이 비정형 config를 손상시킬 수 있음. **미티게이션**: 인식 불가 패턴/복수 매칭 시 None → Guide fallback + 항상 backup 생성
- **Risk: Backup 파일명 변경**: 기존 Ghostty backup에도 영향. **미티게이션**: Ghostty config 파일명이 `config`(확장자 없음)이므로 동일 결과, 하위 호환

## Sources & References

### Origin

- **Origin document:** [docs/brainstorms/2026-03-22-wezterm-support-requirements.md](docs/brainstorms/2026-03-22-wezterm-support-requirements.md) — 핵심 결정: 핵심 색상만 포함 (YAGNI), ModifyConfig 방식, config 디렉토리 감지, 원본 이름 유지

### Internal References

- Ghostty 타겟 (가장 가까운 패턴): `src/target/ghostty.rs`
- Target dispatch: `src/target/mod.rs:46-126`
- Target enum: `src/cli.rs:85-93`
- Orchestrator pipeline: `src/main.rs:300-362` (`write_link_and_save`)
- Atomic write: `src/store.rs`
- Test fixture: `src/ir.rs:272-322` (`make_test_ir`)
- iTerm2 0o644 패턴: `src/target/iterm2.rs:43-47`

### External References

- WezTerm config files: https://wezterm.org/config/files.html
- WezTerm color scheme 문서: https://wezterm.org/config/appearance.html
- WezTerm custom colors 위치: `~/.config/wezterm/colors/*.toml`
- WezTerm config_builder: https://wezterm.org/config/lua/wezterm/config_builder.html
- Real-world dotfiles 조사: rescenic, prabirshrestha, jdhao, ayamir, bennypowers

### Institutional Learnings

- iTerm2 구현에서 배운 점: path DRY 원칙, atomic_write 후 0o644 권한, link_path() 명시 구현 필요
- Code review에서 배운 점: target이 도메인 로직 소유, atomic symlink, LinkResult::Conflict vs Failed 구분, XDG validation 필터

### Research Insights (Deepen)

- **Lua config 패턴**: Config builder (`config.color_scheme = "X"`) 가 현재 가장 보편적. Return table (`color_scheme = "X"`) 도 빈번. 조건부/동적 패턴은 Guide fallback이 올바른 접근
- **Security**: Lua는 Turing-complete 언어이므로 Ghostty(plain text config)와 달리 string injection 벡터 존재. `lua_escape_string()`이 필수
- **Config resolution**: WezTerm은 XDG 경로 우선, `~/.wezterm.lua`는 fallback. macOS 전용 경로 미사용
- **TOML format**: iTerm2-Color-Schemes, Catppuccin 등 실제 파일과 plan 출력 포맷 100% 일치 확인
- **Simplicity**: regex crate 불필요 (Ghostty의 `starts_with`/`split_once` 패턴으로 충분), phase 간소화 적용
