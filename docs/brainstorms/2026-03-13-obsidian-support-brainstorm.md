# Brainstorm: Obsidian Theme Support

**Date:** 2026-03-13
**Status:** Complete
**Participants:** User, Claude

## What We're Building

Chromaport에 Obsidian을 새로운 타겟으로 추가하여, VS Code/Cursor 에디터 테마를 Obsidian 테마(CSS + manifest.json)로 변환할 수 있게 한다. Obsidian vault 경로를 `obsidian.json`에서 자동 감지하고, 여러 vault가 있을 경우 선택 UI를 제공한다.

## Why This Approach

### obsidian.json 파싱을 선택한 이유

- Obsidian vault는 사용자가 어디에든 만들 수 있어 고정 경로 감지가 불가능
- Obsidian 앱은 `~/Library/Application Support/obsidian/obsidian.json`에 열어본 vault 목록을 JSON으로 관리
- 이 파일을 파싱하면 모든 vault 경로를 한 번에 확인 가능
- 홈 디렉토리 스캔 대비 빠르고 정확

### Vault에 직접 쓰기를 선택한 이유

- Obsidian은 vault 내부의 `.obsidian/themes/<name>/` 경로에서 테마를 로드
- vault가 여러 개이므로 심링크 기반 접근은 복잡도만 증가
- 직접 쓰기가 가장 단순하고, Obsidian 커뮤니티 테마 설치 방식과도 동일

### CSS 매핑 범위를 선택한 이유

- Obsidian은 400+ CSS 변수를 제공하며, 핵심 색상만으로는 밋밋한 결과물
- ThemeIR의 기본 색상 + HSL 조정으로 파생 색상을 계산하여 ~20개 변수를 매핑
- 색상 파생은 기존 `src/color.rs`의 HSL/OKLCH 변환 유틸리티 활용
- 결과물이 부족하면 이후 heading, link, tag 등으로 확장 가능

## Key Decisions

### 1. Vault 감지: obsidian.json 파싱

- macOS: `~/Library/Application Support/obsidian/obsidian.json`
- JSON 구조: `{ "vaults": { "<id>": { "path": "<absolute_path>", "ts": <timestamp>, "open": <bool> } } }`
- `detect()`: 이 파일이 존재하고 유효한 vault가 1개 이상이면 `true`
- vault 경로 존재 여부도 검증 (삭제된 vault 필터링)

### 2. Multi-vault: 선택 UI 제공

- vault가 1개면 자동 선택
- vault가 여러 개면 dialoguer로 목록 표시, 사용자가 선택
- 기존 에디터/테마 선택 UI와 동일한 UX 패턴
- `apply` 명령어에서도 동일하게 vault 선택 UI 제공

### 3. 적용 방식: Vault에 직접 쓰기

- 출력 경로: `{vault}/.obsidian/themes/chromaport-{slug}/`
- 파일 구성:
  - `manifest.json`: name, version, minAppVersion, author, authorUrl
  - `theme.css`: `.theme-dark` / `.theme-light` 셀렉터로 색상 변수 정의
- `link()`: `LinkResult::NotApplicable` (Superset과 동일, 심링크 불필요)
- `~/chromaport/themes/obsidian/chromaport-{slug}/`에도 theme.css + manifest.json 저장 (기존 타겟 패턴 유지, IR은 별도 `~/chromaport/themes/{slug}.json`)

### 4. CSS 매핑: 핵심 ~20개 변수

ThemeIR → Obsidian CSS 변수 매핑:

| Obsidian CSS 변수 | ThemeIR 필드 | 파생 방법 |
|---|---|---|
| `--color-base-00` | `background` | 직접 매핑 |
| `--color-base-10` | `background` | HSL lightness ±3% |
| `--color-base-20` | `input_bg` | 직접 매핑 |
| `--color-base-25` | `sidebar_bg` | 직접 매핑 |
| `--color-base-30` | `border` | 직접 매핑 |
| `--color-base-40` | `border` | HSL lightness ±10% |
| `--color-base-50` | `muted_fg` | 직접 매핑 |
| `--color-base-60` | `muted_fg` | HSL lightness ±8% |
| `--color-base-70` | `sidebar_fg` | 직접 매핑 |
| `--color-base-100` | `foreground` | 직접 매핑 |
| `--accent-h` | `accent` | HSL hue 추출 |
| `--accent-s` | `accent` | HSL saturation 추출 |
| `--accent-l` | `accent` | HSL lightness 추출 |
| `--background-primary` | `background` | 직접 매핑 |
| `--background-secondary` | `sidebar_bg` | 직접 매핑 |
| `--text-normal` | `foreground` | 직접 매핑 |
| `--text-muted` | `muted_fg` | 직접 매핑 |
| `--text-faint` | `muted_fg` | HSL lightness ±15% |
| `--text-highlight-bg` | `selection_bg` | 직접 매핑 |
| `--interactive-accent` | `accent` | 직접 매핑 |
| `--code-normal` | `chart_colors[0]` | 직접 매핑 |
| `--code-background` | `background` | HSL lightness ±5% (dark: 어둡게, light: 밝게) |

이 ~20개가 초기 구현 범위. 결과물 검토 후 heading, link, tag 등에 `chart_colors`를 활용한 확장 가능.

### 5. OS 지원: macOS 우선

- 초기 구현은 macOS만 지원
- `detect()`에서 macOS가 아닌 경우 `false` 반환
- 향후 Linux (`~/.config/obsidian/`), Windows (`%APPDATA%/obsidian/`) 추가 가능
- OS별 경로 분기는 별도 함수로 분리하여 확장 용이하게 설계

### 6. post_write_action: 가이드 메시지

- Obsidian에서 테마 활성화 방법 안내:
  - Settings → Appearance → Themes → "chromaport-{slug}" 선택
- config 파일 자동 수정은 하지 않음 (Obsidian이 자체 관리)
- `PostWriteAction::Guide { message }` 사용

### 7. 다크/라이트 모드 처리

- ThemeIR의 `theme_type` (Dark/Light)에 따라 해당 모드의 CSS만 생성
- `.theme-dark` 또는 `.theme-light` 셀렉터 사용
- 하나의 테마 파일에 한 가지 모드만 포함 (Obsidian 커뮤니티 테마 관행)

## 테마 출력 예시

### manifest.json

```json
{
  "name": "chromaport-one-dark",
  "version": "1.0.0",
  "minAppVersion": "1.0.0",
  "author": "chromaport",
  "authorUrl": "https://github.com/<owner>/chromaport"
}
```

### theme.css

```css
.theme-dark {
  --color-base-00: #282c34;
  --color-base-10: #2c313a;
  --color-base-20: #303642;
  --color-base-25: #21252b;
  --color-base-30: #3e4451;
  --color-base-40: #4b5263;
  --color-base-50: #5c6370;
  --color-base-60: #737984;
  --color-base-70: #9da5b4;
  --color-base-100: #abb2bf;
  --accent-h: 220;
  --accent-s: 68%;
  --accent-l: 51%;
  --background-primary: #282c34;
  --background-secondary: #21252b;
  --text-normal: #abb2bf;
  --text-muted: #5c6370;
  --text-highlight-bg: #3e4451;
  --interactive-accent: #528bff;
  --code-normal: #e06c75;
  --code-background: #252930;
}
```

## 변경 영향 범위

| 파일 | 변경 내용 |
|---|---|
| `src/cli.rs` | `Target` enum에 `Obsidian` 추가 |
| `src/target/obsidian.rs` | 신규: Obsidian 타겟 구현 (detect, write, link, post_write_action) |
| `src/target/mod.rs` | `pub mod obsidian;` 추가, match arms 추가, `Target::all()` 업데이트 |
| `Cargo.toml` | version bump (minor) |
| `tests/cli.rs` | Obsidian 관련 테스트 추가 |

## Resolved Questions

- **Vault 감지 방식**: obsidian.json 파싱 (macOS: `~/Library/Application Support/obsidian/obsidian.json`)
- **Multi-vault 처리**: 여러 vault 발견 시 dialoguer 선택 UI 제공, 1개면 자동 선택
- **적용 방식**: vault의 `.obsidian/themes/` 디렉토리에 직접 쓰기 (심링크 없음)
- **CSS 매핑 수준**: 핵심 ~20개 변수. ThemeIR 직접 매핑 + HSL lightness 조정으로 파생. 결과 확인 후 확장 가능
- **OS 지원**: macOS 우선. 향후 Linux/Windows 추가 가능하도록 경로 분기 분리
- **`apply` 명령어에서 vault 선택**: 매번 vault 선택 UI를 보여준다. 상태 관리 없이 단순하게 유지.
- **authorUrl**: chromaport GitHub repo URL 사용. 일관성 있고 단순함.
