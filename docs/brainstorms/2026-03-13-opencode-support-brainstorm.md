# Brainstorm: OpenCode Desktop Theme Support

**Date:** 2026-03-13
**Status:** Reviewed

## What We're Building

chromaport에 OpenCode Desktop을 **양방향(import + export)** 지원하는 기능을 추가한다.

- **Export (Target):** VS Code/Cursor 테마를 OpenCode 테마 JSON 포맷(`~/.config/opencode/themes/*.json`)으로 변환
- **Import (Source):** OpenCode 테마를 읽어 ThemeIR로 변환 후 Ghostty/Warp/Superset 등으로 내보내기

OpenCode 테마는 semantic UI 컬러 체계(syntax, diff, markdown 등)를 사용하므로, 기존 터미널 중심(ANSI palette)의 ThemeIR을 확장해야 한다.

## Why This Approach

### ThemeIR 확장 전략 선택 이유

현재 ThemeIR은 ANSI 16색 + UI 컬러 10종 + chart_colors 5종만 저장한다. OpenCode는 syntax 9종, diff 10+종, markdown 14종의 세분화된 시맨틱 컬러를 요구한다.

**선택: ThemeIR에 optional 필드 추가**

- `syntax: Option<SyntaxColors>` — VS Code tokenColors에서 직접 추출
- `diff: Option<DiffColors>` — VS Code git decoration colors에서 추출
- 기존 타겟(Ghostty, Warp, Superset)은 이 필드를 무시하므로 하위 호환성 유지
- 향후 다른 semantic-color 기반 타겟에도 재활용 가능

**대안 기각:**
- 타겟에서 파생: chart_colors 5개로 syntax 9종을 추측하면 정확도가 낮음
- 하이브리드: 점진적이지만 두 번 작업하게 됨

### 양방향 지원 이유

OpenCode는 자체 테마 생태계를 갖고 있고(tokyonight, catppuccin 등 빌트인), 사용자가 이를 터미널(Ghostty/Warp)에도 적용하고 싶을 수 있다. Editor enum에 OpenCode variant를 추가하면 기존 `chromaport --editor opencode --target ghostty` 흐름으로 자연스럽게 통합된다.

## Key Decisions

### 1. ThemeIR 확장 (syntax + diff optional 필드)

```rust
pub struct ThemeIR {
    // ... 기존 필드 유지 ...

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syntax: Option<SyntaxColors>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffColors>,
}

pub struct SyntaxColors {
    pub comment: HexColor,
    pub keyword: HexColor,
    pub function: HexColor,
    pub variable: HexColor,
    pub string: HexColor,
    pub number: HexColor,
    pub r#type: HexColor,
    pub operator: HexColor,
    pub punctuation: HexColor,
}

pub struct DiffColors {
    pub added: HexColor,
    pub removed: HexColor,
    pub context: HexColor,
    pub hunk_header: HexColor,
}
```

- `Option`으로 감싸서 기존 저장된 IR JSON과 하위 호환
- converter.rs에서 VS Code tokenColors → SyntaxColors 매핑 추가
- converter.rs에서 VS Code gitDecoration/diffEditor colors → DiffColors 매핑 추가

### 2. Editor enum에 OpenCode 추가 (Import)

```rust
pub enum Editor {
    Vscode,
    Cursor,
    OpenCode,  // NEW
}
```

- reader.rs에 `scan_opencode_themes()` 추가: `~/.config/opencode/themes/*.json` 스캔
- converter.rs에 `convert_opencode()` 추가: OpenCode JSON → ThemeIR 역변환
- OpenCode는 extensions 구조가 아닌 단순 JSON 파일이므로 reader 로직이 단순

### 3. Target에 OpenCode 추가 (Export)

```rust
pub enum Target {
    Superset,
    Warp,
    Ghostty,
    OpenCode,  // NEW
}
```

- `src/target/opencode.rs` 신규 생성
- detect(): `~/.config/opencode/` 존재 여부
- write(): ThemeIR → OpenCode JSON 직렬화
- link(): NotApplicable (XDG에 직접 작성)
- post_write_action(): tui.json의 theme 필드 자동 수정 (Ghostty 패턴)

### 4. 단일 모드 출력 (Dark/Light)

ThemeIR의 `theme_type`에 따라 해당 모드의 컬러만 출력. OpenCode의 `{dark, light}` dual variant는 사용하지 않음. 사용자가 dark/light 테마를 각각 별도로 변환하면 됨.

### 5. defs 섹션 활용

OpenCode JSON 출력 시 `defs`에 기본 색상을 정의하고 `theme`에서 참조:

```json
{
  "$schema": "https://opencode.ai/theme.json",
  "defs": {
    "bg": "#1e1e1e",
    "fg": "#d4d4d4",
    "accent": "#0078d4",
    "muted": "#858585"
  },
  "theme": {
    "background": "bg",
    "text": "fg",
    "primary": "accent",
    "textMuted": "muted",
    "secondary": "#3e3e3e",
    "accent": "accent",
    "syntaxKeyword": "#e06c75",
    ...
  }
}
```

### 6. XDG global 경로 배치

- Export 경로: `~/.config/opencode/themes/{slug}.json`
- XDG_CONFIG_HOME 환경변수 존중
- 중앙 저장소: `~/.config/chromaport/themes/opencode/{slug}.json`

### 7. tui.json 자동 수정 (Post-write)

Ghostty 패턴과 동일:
- `~/.config/opencode/tui.json` 존재 시 → `theme` 필드 수정 (ModifyConfig)
- 파일 없을 시 → 새로 생성 (CreateConfig)
- 사용자 확인 프롬프트 포함

## ThemeIR → OpenCode 매핑 상세

| OpenCode Property | ThemeIR Source | 비고 |
|---|---|---|
| **필수** | | |
| `primary` | `accent` | |
| `secondary` | `sidebar_bg` | border보다 semantic으로 가까움 |
| `accent` | `accent` | primary와 동일 |
| `text` | `foreground` | |
| `textMuted` | `muted_fg` | |
| `background` | `background` | |
| **UI 선택** | | |
| `error` | `terminal.normal.red` | |
| `warning` | `terminal.normal.yellow` | |
| `success` | `terminal.normal.green` | |
| `info` | `terminal.normal.blue` | |
| `border` | `border` | |
| `borderActive` | `accent` | 활성 상태 border는 accent 활용 |
| `borderSubtle` | `sidebar_bg` 밝기 조정 | color.rs로 border보다 연한 톤 파생 |
| `backgroundPanel` | `sidebar_bg` | |
| `backgroundElement` | `input_bg` | |
| **Syntax** | | |
| `syntaxComment` | `syntax.comment` | tokenColors에서 추출 |
| `syntaxKeyword` | `syntax.keyword` | |
| `syntaxFunction` | `syntax.function` | |
| `syntaxVariable` | `syntax.variable` | |
| `syntaxString` | `syntax.string` | |
| `syntaxNumber` | `syntax.number` | |
| `syntaxType` | `syntax.type` | |
| `syntaxOperator` | `syntax.operator` | |
| `syntaxPunctuation` | `syntax.punctuation` | |
| **Diff** | | |
| `diffAdded` | `diff.added` | gitDecoration colors에서 |
| `diffRemoved` | `diff.removed` | |
| `diffContext` | `diff.context` | |
| `diffHunkHeader` | `diff.hunk_header` | |
| **Markdown** | | |
| `markdownHeading` | `syntax.keyword` | 파생 |
| `markdownCode` | `syntax.string` | 파생 |
| `markdownLink` | `accent` | 파생 |
| **미지원 (v1)** | | |
| `diffHighlightAdded/Removed` | — | diff bg 변형; 향후 확장 가능 |
| `diffAddedBg`, `diffRemovedBg`, `diffContextBg` | — | 배경색 변형; 향후 확장 가능 |
| `diffLineNumber`, `diff*LineNumberBg` | — | |
| `markdownBlockQuote` ~ `markdownImage` 등 11종 | — | 나머지 markdown 속성은 OpenCode 기본값에 위임 |

## OpenCode → ThemeIR 매핑 (Import)

| ThemeIR Field | OpenCode Source | 비고 |
|---|---|---|
| `background` | `theme.background` | |
| `foreground` | `theme.text` | |
| `accent` | `theme.accent` 또는 `theme.primary` | |
| `muted_fg` | `theme.textMuted` | |
| `border` | `theme.border` | |
| `sidebar_bg` | `theme.backgroundPanel` | |
| `input_bg` | `theme.backgroundElement` | |
| `terminal.normal.red` | `theme.error` | |
| `terminal.normal.green` | `theme.success` | |
| `terminal.normal.yellow` | `theme.warning` | |
| `terminal.normal.blue` | `theme.info` | |
| `syntax.keyword` | `theme.syntaxKeyword` | |
| ANSI palette | 색상 팔레트에서 파생 | color.rs의 derive_palette 활용 |

### Import 시 비-hex 컬러 처리

OpenCode 테마는 hex 외에 3가지 컬러 표현을 지원한다. reader에서 ThemeIR(HexColor 전용)로 변환 시 다음 순서로 처리:

1. **defs 참조 해석**: `"background": "base"` → defs에서 `"base": "#1e1e1e"` 찾아 치환. 재귀 참조 방지를 위해 1단계만 resolve.
2. **ANSI integer 변환**: `0-255` 정수값 → 표준 256-color 팔레트 hex로 변환. `color.rs`에 `ansi256_to_hex(u8) -> HexColor` 유틸 추가.
3. **"none" 처리**: terminal 기본색 의미 → 해당 테마의 `background`(배경 계열) 또는 `text`(전경 계열)로 대체.
4. **dark/light 객체**: `{"dark": "#000", "light": "#fff"}` → ThemeIR의 theme_type에 맞는 쪽 선택.

## Scope & Complexity

### 변경 대상 파일

| 파일 | 변경 유형 | 규모 |
|---|---|---|
| `src/ir.rs` | SyntaxColors, DiffColors 구조체 추가 | ~60 LOC |
| `src/converter.rs` | tokenColors → SyntaxColors/DiffColors 추출 + convert_opencode() 역변환 | ~160 LOC |
| `src/cli.rs` | Editor::OpenCode, Target::OpenCode 추가 | ~10 LOC |
| `src/target/mod.rs` | OpenCode dispatch 추가 | ~20 LOC |
| `src/target/opencode.rs` | **신규** — detect/write/link/post_write | ~200 LOC |
| `src/reader.rs` | scan_opencode_themes() + defs/ANSI/none 해석 | ~100 LOC |
| `src/color.rs` | ansi256_to_hex() 유틸 추가 | ~30 LOC |
| `src/main.rs` | OpenCode editor/target 분기 | ~20 LOC |
| `src/apply.rs` | Target::all() 업데이트 반영 | ~5 LOC |

**총 예상:** ~600 LOC 추가/수정

## Open Questions

*모든 질문이 해결되었습니다.*

## Resolved Questions

1. **IR 확장 전략** → ThemeIR에 optional syntax/diff 필드 추가
2. **Import 지원** → 양방향 (Import + Export) 모두 지원
3. **Dark/Light 처리** → 단일 모드 출력 (theme_type에 따라)
4. **배치 경로** → XDG global (`~/.config/opencode/themes/`)
5. **Import 아키텍처** → Editor enum에 OpenCode variant 추가
6. **defs 활용** → 기본 색상을 defs에 정의하고 theme에서 참조
7. **Post-write** → tui.json theme 필드 자동 수정 (Ghostty 패턴)
8. **비-hex 컬러 처리** → defs 해석 → ANSI 256-color 변환 → "none"은 bg/fg 대체 → dark/light 객체는 theme_type으로 선택
9. **secondary 매핑** → `sidebar_bg` (border보다 semantic으로 가까움)
10. **미지원 속성** → diff bg 변형, markdown 11종 등은 v1에서 미지원, OpenCode 기본값에 위임
