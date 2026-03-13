---
title: "feat: Add OpenCode Desktop bidirectional theme support"
type: feat
status: active
date: 2026-03-13
deepened: 2026-03-13
reviewed: 2026-03-13
origin: docs/brainstorms/2026-03-13-opencode-support-brainstorm.md
---

# feat: Add OpenCode Desktop bidirectional theme support

## Review Summary

**Reviewed on:** 2026-03-13
**Reviewers:** Architecture Strategist, Pattern Recognition, Performance Oracle, Security Sentinel, Code Simplicity

### Key Changes from Technical Review
1. **ThemeExtensions 래퍼 제거**: `Option<SyntaxColors>` / `Option<DiffColors>`를 ThemeIR에 직접 추가 (기존 `cursor_accent`, `created_at` 패턴과 일관)
2. **Converter 서브모듈 분할 취소**: `converter.rs` 유지 + `converter_opencode.rs` flat sibling file 생성 (455줄 파일에 디렉토리 리스트럭처링 불필요)
3. **출력 포맷팅 위치 수정**: ThemeIR→OpenCode JSON 포맷팅은 `target/opencode.rs`에 (converter는 입력 방향만, target은 출력 방향)
4. **handle_post_write_action 추출 별도 PR로 분리**: target-agnostic 함수라 4번째 타겟 추가 시 변경 불필요. Feature PR 크기 축소.
5. **ANSI lookup table → warn+fallback**: 실제 OpenCode 테마가 ANSI integer를 사용하지 않음. 3줄 fallback으로 충분.
6. **보안 강화**: `store::is_regular_file()` (symlink 방지), `MAX_THEME_BYTES=1MB`, `MAX_THEME_FILES=256`, `theme_slug()` 필수
7. **단일 패스 tokenColors 통합 취소**: 별도 함수로 유지 (Performance — 데이터 크기 대비 복잡성 불필요)

### v1 Scope Reduction (총 ~180 LOC 절감, 원래 600 → ~420)
| 제거/연기 항목 | 사유 |
|---|---|
| ThemeExtensions 래퍼 | YAGNI — 2개 필드에 wrapper struct 불필요, Option이 Rust의 확장 메커니즘 |
| Converter 서브모듈 디렉토리 | 455줄 파일에 과도한 구조 변경 + import 경로 변경 부담 |
| handle_post_write_action 추출 | target-agnostic이라 4번째 타겟과 무관. 별도 chore: 커밋 |
| ANSI 16색 lookup table | 실제 사용하는 테마 없음. warn+fallback으로 대체 |
| defs export 생성 | 프로그래매틱 생성 파일에 indirection 불필요 |
| ANSI 16-255 변환 | 실제 OpenCode 테마가 사용하지 않음 |
| dark/light variant 객체 | 테마 파일은 단일 variant |
| borderSubtle 파생 | sidebar_bg 직접 사용으로 충분 |
| 단일 패스 tokenColors 통합 | 성능 차이 무의미, 코드 복잡성만 증가 |

---

## Overview

chromaport에 OpenCode Desktop을 양방향(import + export) 지원한다. VS Code/Cursor 테마를 OpenCode JSON 포맷으로 내보내고, OpenCode 테마를 읽어 Ghostty/Warp/Superset 등으로 내보낼 수 있다.

이를 위해 ThemeIR에 syntax/diff optional 필드를 추가하고, `Editor::OpenCode` + `Target::OpenCode`를 구현한다.

(see brainstorm: docs/brainstorms/2026-03-13-opencode-support-brainstorm.md)

## Problem Statement / Motivation

OpenCode Desktop은 semantic UI 컬러 체계(syntax 9종, diff 10+종, markdown 14종)를 사용하는 코드 에디터/터미널이다. 사용자가 VS Code의 테마를 OpenCode에서도 쓰고 싶거나, OpenCode의 빌트인 테마(tokyonight, catppuccin 등)를 Ghostty/Warp에 적용하고 싶은 수요가 있다.

현재 ThemeIR은 터미널 중심(ANSI palette + UI 10종 + chart_colors 5종)으로 OpenCode의 세분화된 시맨틱 컬러를 표현할 수 없다.

## Proposed Solution

### Phase A: ThemeIR 확장 + OpenCode Target (Export)

ThemeIR에 `syntax`/`diff` optional 필드를 추가하고, VS Code converter에서 추출한 뒤, `src/target/opencode.rs`로 OpenCode JSON을 생성한다. tui.json 자동 수정 post-write action을 제공한다.

### Phase B: OpenCode Editor (Import) + 마무리

`Editor::OpenCode`를 추가하고, `~/.config/opencode/themes/*.json`에서 테마를 스캔하는 reader를 구현한다. defs 참조 해석, ANSI integer warn+fallback, "none" 인라인 처리를 지원한다. 버전 bump, 에러 메시지 정리.

## Technical Approach

### Architecture

```
[기존 흐름]
VS Code/Cursor → reader.rs → converter.rs → ThemeIR → target/*.rs → Ghostty/Warp/Superset

[추가되는 흐름 - Export]
VS Code/Cursor → reader.rs → converter.rs → ThemeIR(+syntax/diff) → target/opencode.rs → OpenCode JSON

[추가되는 흐름 - Import]
OpenCode JSON → reader.rs(new) → converter_opencode.rs(new) → ThemeIR → target/*.rs → 모든 타겟
```

**아키텍처 경계 원칙 (Technical Review 확인):**
- **converter** = 소스 → ThemeIR (입력 방향만). `converter.rs`(VS Code), `converter_opencode.rs`(OpenCode)
- **target** = ThemeIR → 출력 포맷 (출력 방향만). `target/opencode.rs`에 `format_opencode_theme()` 배치
- `converter.rs`는 그대로 유지 (455줄, 관리 가능). 디렉토리 분할 하지 않음.
- OpenCode import 흐름은 `detect_editors()` 결과에 끼워넣지 않고 `main.rs`에서 완전 별도 경로

**detect_editors()에서 OpenCode 분리:**

`detect_editors()`는 `Vec<(Editor, PathBuf, PathBuf)>` 반환 (extensions_dir, settings_path). OpenCode에는 이 개념이 없으므로 시맨틱 불일치 발생. OpenCode import는 별도 경로:

```rust
// main.rs — OpenCode는 detect_editors() 흐름과 완전 별도
match editor {
    Editor::Vscode | Editor::Cursor => {
        let editors = reader::detect_editors();
        // ThemeReader 기반 흐름
    }
    Editor::OpenCode => {
        // reader::scan_opencode_themes() → 선택 → convert_opencode()
    }
}
```

### Implementation Phases

#### Phase A: ThemeIR 확장 + OpenCode Target (Export)

**A-1. ThemeIR 확장**

**변경 파일: `src/ir.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffColors {
    pub added: HexColor,
    pub removed: HexColor,
    pub context: HexColor,
    pub hunk_header: HexColor,
}

pub struct ThemeIR {
    // ... 기존 필드 유지 ...

    /// Semantic syntax colors (VS Code tokenColors에서 추출, OpenCode에서 직접 사용)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syntax: Option<SyntaxColors>,

    /// Diff/git decoration colors
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffColors>,
}
```

**선택 근거 (Technical Review):** `cursor_accent: Option<HexColor>`, `selection_bg: Option<HexColor>`, `created_at: Option<String>` 등 기존 ThemeIR의 Optional 필드 패턴과 일관. syntax/diff는 OpenCode 전용이 아닌 범용 시맨틱 데이터(Zed, Helix 등도 사용 가능)이므로 IR에 직접 배치가 적절.

### Serde Backward Compat

- `Option<T>`는 JSON에서 필드가 없으면 자동으로 `None`으로 역직렬화됨
- `skip_serializing_if`로 기존 파일에 null 필드가 추가되지 않음
- `deny_unknown_fields` 절대 사용 금지 (forward compat 파괴)
- ThemeIR 확인 완료: `deny_unknown_fields` 사용하지 않음

**변경 파일: `src/ir.rs` test_fixtures** — `make_test_ir()`에 `syntax: None, diff: None` 추가

**A-2. VS Code Converter — syntax/diff 추출 추가**

**변경 파일: `src/converter.rs`** (기존 파일에 함수 추가, 파일 이동 없음)

syntax/diff 추출을 `extract_chart_colors`와 **별도 함수**로 구현 (Performance Review: 단일 패스 통합은 성능 차이 무의미, 코드 복잡성만 증가):

```rust
/// VS Code tokenColors에서 SyntaxColors 추출
fn extract_syntax_colors(token_colors: &[Value], theme_type: ThemeType, ...) -> SyntaxColors { ... }

/// VS Code colors 객체에서 DiffColors 추출
fn extract_diff_colors(colors: &HashMap<String, String>, theme_type: ThemeType, ...) -> DiffColors { ... }
```

VS Code tokenColors 스코프 매핑:

| SyntaxColors field | VS Code scope(s) | Dark fallback | Light fallback |
|---|---|---|---|
| `comment` | `comment`, `comment.line`, `comment.block` | `#6A9955` | `#008000` |
| `keyword` | `keyword`, `storage.type`, `keyword.operator` | chart_colors[0] | chart_colors[0] |
| `function` | `entity.name.function`, `support.function` | chart_colors[2] | chart_colors[2] |
| `variable` | `variable`, `variable.other` | foreground | foreground |
| `string` | `string`, `string.template` | chart_colors[1] | chart_colors[1] |
| `number` | `constant.numeric`, `constant.language` | chart_colors[3] | chart_colors[3] |
| `type` | `entity.name.type`, `support.type` | chart_colors[4] | chart_colors[4] |
| `operator` | `keyword.operator`, `punctuation.definition` | foreground | foreground |
| `punctuation` | `punctuation`, `punctuation.separator` | muted_fg | muted_fg |

VS Code diff color 매핑:

| DiffColors field | VS Code key(s) | Dark fallback | Light fallback |
|---|---|---|---|
| `added` | `gitDecoration.addedResourceForeground` | `#81B88B` | `#587C0C` |
| `removed` | `gitDecoration.deletedResourceForeground` | `#C74E39` | `#AD0707` |
| `context` | `diffEditor.unchangedRegionBackground` | muted_fg | muted_fg |
| `hunk_header` | `diffEditor.hunkHeaderBackground` | accent | accent |

**A-3. OpenCode Target (Export)**

**변경 파일: `src/target/opencode.rs`** (신규)

```rust
// 핵심 함수 시그니처 — mod.rs dispatch 계약에 맞춤
pub fn detect() -> bool;
pub fn write(ir: &ThemeIR) -> Result<PathBuf>;
pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf>;
pub fn link(ir: &ThemeIR, written_path: &Path) -> LinkResult;
pub fn post_write_action(ir: &ThemeIR, _written_path: &Path) -> PostWriteAction;

/// ThemeIR → OpenCode JSON 포맷팅 (private, target 모듈 내부)
/// Ghostty의 format_ghostty_theme(), Superset의 ir_to_json() 패턴
fn format_opencode_theme(ir: &ThemeIR) -> serde_json::Value { ... }
```

**OpenCode JSON 출력 포맷 (v1 — flat hex, defs 없음):**

```json
{
  "$schema": "https://opencode.ai/theme.json",
  "theme": {
    "primary": "#0078D4",
    "secondary": "#252526",
    "accent": "#0078D4",
    "text": "#D4D4D4",
    "textMuted": "#858585",
    "background": "#1E1E1E",
    "error": "#CD3131",
    "warning": "#E5E510",
    "success": "#0DBC79",
    "info": "#2472C8",
    "border": "#3E3E3E",
    "borderActive": "#0078D4",
    "borderSubtle": "#252526",
    "backgroundPanel": "#252526",
    "backgroundElement": "#3C3C3C",
    "syntaxComment": "#6A9955",
    "syntaxKeyword": "#E06C75",
    "syntaxFunction": "#61AFEF",
    "syntaxVariable": "#D4D4D4",
    "syntaxString": "#98C379",
    "syntaxNumber": "#C678DD",
    "syntaxType": "#56B6C2",
    "syntaxOperator": "#D4D4D4",
    "syntaxPunctuation": "#858585",
    "diffAdded": "#81B88B",
    "diffRemoved": "#C74E39",
    "diffContext": "#858585",
    "diffHunkHeader": "#0078D4",
    "markdownHeading": "#E06C75",
    "markdownCode": "#98C379",
    "markdownLink": "#0078D4"
  }
}
```

**ThemeIR → OpenCode 매핑 규칙:**

- 필수 6개: primary←accent, secondary←sidebar_bg, accent←accent, text←foreground, textMuted←muted_fg, background←background
- UI: error←terminal.normal.red, warning←terminal.normal.yellow, success←terminal.normal.green, info←terminal.normal.blue, border←border, **borderActive←accent**, **borderSubtle←sidebar_bg**, backgroundPanel←sidebar_bg, backgroundElement←input_bg
- Syntax 9종: `ir.syntax`가 Some이면 직접 매핑, **None이면 chart_colors + foreground + muted_fg에서 인라인 파생** (legacy IR 호환)
- Diff 4종: `ir.diff`가 Some이면 직접 매핑, **None이면 ANSI red/green + muted_fg + accent에서 인라인 파생**
- Markdown 3종: keyword→heading, string→code, accent→link
- 미지원 속성(diff bg 변형, markdown 11종): 출력하지 않음, OpenCode 기본값에 위임

**tui.json 수정 (Ghostty config 패턴):**
- `opencode_config_dir()`: XDG 검증 필터 통일 (`.filter(|s| !s.is_empty() && Path::new(s).is_absolute())`)
- tui.json 경로: `{config_dir}/tui.json`
- 존재 시: `serde_json::from_str` → `theme` 키 수정 → `serde_json::to_string_pretty` (다른 키 보존)
- **invalid JSON일 경우**: `PostWriteAction::Guide` fallback (덮어쓰지 않음)
- 미존재 시: `CreateConfig { path, content }`

**Symlink:**
- 중앙 저장소: **`~/chromaport/themes/opencode/{slug}.json`** (`chromaport_themes_dir("opencode")` 사용)
- 타겟 경로: `~/.config/opencode/themes/{slug}.json`
- `link()`: `store::create_symlink(written_path, target_path, false)` (Ghostty 패턴)
- **파일명**: `theme_slug()` 사용 필수 (Security — 경로 순회 방지. Ghostty의 lenient `theme_filename`이 아닌 `store::theme_slug()`)

**A-4. CLI + dispatch 업데이트**

| 파일 | 변경 |
|---|---|
| `src/cli.rs` | `Target::OpenCode` variant 추가 |
| `src/target/mod.rs` | `pub mod opencode;` + 6개 match arm + `Target::all() → [Target; 4]` + `display_name` |
| `src/main.rs` | 에러 메시지 업데이트 (하드코딩된 "Superset, Warp, or Ghostty" → OpenCode 포함) |
| `src/apply.rs` | 에러 메시지 업데이트 |
| `src/interactive.rs` | `select_target` 에러 메시지 |

**패턴 준수 (Pattern Recognition 확인):**
- **출력 포맷 함수**: `format_opencode_theme()` → `target/opencode.rs` private (Ghostty의 `format_ghostty_theme()` 패턴)
- **파일명**: `{slug}.json` (Warp 패턴)
- **Config dir**: `opencode_config_dir()` (Ghostty 패턴)
- **post_write_action 시그니처**: `(ir: &ThemeIR, _written_path: &Path)` — dispatch 계약 준수
- **테스트**: `ir::test_fixtures::make_test_ir()` 공유 fixture 사용

**Phase A 성공 기준:**
- [ ] 기존 저장된 IR JSON 파일이 정상 역직렬화됨 (serde backward compat)
- [ ] VS Code 테마 변환 시 `ir.syntax`/`ir.diff` 필드가 채워짐
- [ ] `chromaport --editor vscode --target opencode` 정상 동작
- [ ] OpenCode JSON 출력이 `https://opencode.ai/theme.json` 스키마 준수
- [ ] tui.json 자동 수정 (존재/미존재/invalid JSON/기존 theme 4가지 케이스)
- [ ] `chromaport apply`에서 OpenCode 타겟 선택 가능
- [ ] legacy IR (syntax/diff None)도 정상 export (fallback 파생)
- [ ] `cargo test && cargo fmt --check && cargo clippy --all-targets` 통과

---

#### Phase B: OpenCode Editor (Import) + 마무리

**B-1. Editor enum + OpenCode reader**

**변경 파일: `src/cli.rs`**

```rust
#[derive(Clone, ValueEnum, Debug, PartialEq)]
pub enum Editor {
    Vscode,
    Cursor,
    OpenCode,
}
```

**변경 파일: `src/reader.rs`**

```rust
/// OpenCode 설치 감지 (detect_editors()와 별도 — 반환 타입 불일치 때문)
pub fn detect_opencode() -> bool {
    opencode_themes_dir().map(|d| d.exists()).unwrap_or(false)
}

fn opencode_themes_dir() -> Option<PathBuf> {
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())  // XDG 검증 통일
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("opencode").join("themes"))
}

const MAX_OPENCODE_THEME_BYTES: u64 = 1 * 1024 * 1024;  // 1MB (Security: 테마는 1-5KB)
const MAX_THEME_FILES: usize = 256;  // Security: 파일 수 제한

/// OpenCode 테마 디렉토리 스캔
pub fn scan_opencode_themes() -> Result<Vec<ThemeEntry>> {
    let themes_dir = opencode_themes_dir()
        .context("cannot determine OpenCode themes directory")?;
    // 1. read_dir (비재귀)
    // 2. *.json 필터 + store::is_regular_file() 체크 (Security: symlink 방지)
    // 3. MAX_THEME_FILES 제한 (초과 시 warn + 무시)
    // 4. 파일 크기 체크 (MAX_OPENCODE_THEME_BYTES) — metadata.len() 확인
    // 5. JSON 파싱
    // 6. defs 참조 해석 (단일 패스 HashMap, MAX_DEFS_ENTRIES=256 제한)
    // 7. ThemeEntry 생성 (name = 파일명 stem, theme_type = ThemeType 감지)
}
```

### Security (Import) — Technical Review 강화

- **Symlink 방지**: `store::is_regular_file()` 사용 필수 (`symlink_metadata` 기반). `Path::is_file()` 절대 사용 금지 — symlink을 따라가서 임의 파일 읽기 가능
- **File size guard**: `MAX_OPENCODE_THEME_BYTES = 1MB` (10MB → 1MB 하향. 테마 파일은 1-5KB. JSON 메모리 증폭 방지)
- **File count guard**: `MAX_THEME_FILES = 256` (디렉토리에 수천 개 파일로 DoS 방지)
- **MAX_DEFS_ENTRIES = 256**: defs 객체 크기 제한으로 메모리 소진 방지
- **defs-resolved 값 검증**: resolve 후 반드시 `HexColor::parse()` 통과 확인. 실패 시 fallback 사용
- **파일명**: `theme_slug()` 사용 (경로 순회 방지)
- **기존 코드 수정**: `store.rs`의 `list_ir_files`도 `p.is_file()` → `store::is_regular_file()` 수정 (pre-existing 취약점)

**defs 해석 알고리즘 (단일 패스 HashMap — 순환 원천 차단):**
1. `defs` 객체의 모든 키-값을 `HashMap<String, String>`으로 수집 (최대 256개)
2. `theme` 객체의 각 값: 문자열이고 defs에 키로 존재 → defs 값으로 치환
3. **1단계만 resolve** — defs 값이 또 다른 defs 참조여도 무시 (순환 불가 by construction)
4. resolve된 값이 HexColor 파싱 실패 시 → fallback 사용

**B-2. ANSI integer 처리 (warn+fallback)**

**변경 파일: `src/reader.rs`** (scan_opencode_themes 내부)

실제 OpenCode 테마가 ANSI integer를 사용하지 않으므로 v1에서는 lookup table 대신 warn+fallback:

```rust
// reader.rs — 필드 추출 시 인라인 처리
fn resolve_color(value: &Value, defs: &HashMap<String, String>, fallback: &HexColor) -> HexColor {
    match value {
        Value::String(s) if s == "none" => fallback.clone(),
        Value::String(s) => {
            // defs 참조 해석
            let resolved = defs.get(s.as_str()).map(|v| v.as_str()).unwrap_or(s.as_str());
            HexColor::parse(resolved).unwrap_or_else(|_| fallback.clone())
        }
        Value::Number(n) => {
            eprintln!("  Warning: ANSI integer colors not yet supported (value: {}), using fallback", n);
            fallback.clone()
        }
        _ => fallback.clone(),
    }
}
// TODO(v2): ANSI 0-15 lookup table if real themes use integers
// Correct xterm-256 cube formula: if c == 0 { 0 } else { 55 + c * 40 } (NOT c * 51)
```

**B-3. OpenCode Converter (Import)**

**변경 파일: `src/converter_opencode.rs`** (신규 — flat sibling, 서브모듈 아님)

```rust
use crate::ir::*;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn convert_opencode(name: &str, theme: &serde_json::Map<String, Value>) -> Result<ThemeIR> {
    // 1. 필수 6개 필드 추출 (primary, secondary, accent, text, textMuted, background)
    // 2. UI 선택 필드 추출 (error, warning, success, info, border 등 — 없으면 기본값)
    // 3. syntax 9종 추출 → SyntaxColors (syntaxKeyword, syntaxString, ...)
    // 4. diff 4종 추출 → DiffColors
    // 5. ANSI palette 파생: error→red, success→green, warning→yellow, info→blue
    //    나머지 ANSI → color.rs derive_palette(bg, fg, accent)
    // 6. chart_colors: syntax에서 keyword, string, function, number, type 순으로
    // 7. cursor ← accent, selection_bg ← accent 기반 파생
}
```

**B-4. main.rs 분기**

```rust
// main.rs — editor별 완전 별도 경로 (detect_editors() 흐름에 끼워넣지 않음)
match editor {
    Editor::Vscode | Editor::Cursor => {
        let editors = reader::detect_editors();
        // 기존 ThemeReader 흐름
    }
    Editor::OpenCode => {
        // reader::scan_opencode_themes() → 선택 → convert_opencode()
        // detect_editors()와 완전 분리 — PathBuf 시맨틱 불일치 회피
    }
}
```

**B-5. 마무리**

1. **`Cargo.toml`** — version minor bump (feat이므로), description + keywords에 "opencode" 추가
2. **에러 메시지 정리** — "VS Code / Cursor" → "VS Code / Cursor / OpenCode" 전체 업데이트. 하드코딩된 타겟 목록(`main.rs:116`, `apply.rs:46`)도 OpenCode 포함
3. **`--editor opencode --target opencode`** — 허용. 정규화 패스 (defs 해석, "none" 대체). 정보 메시지 출력: "Theme is already in OpenCode format; normalizing."

**Phase B 성공 기준:**
- [ ] `chromaport --editor opencode --target ghostty` 정상 동작
- [ ] defs 참조가 있는 OpenCode 테마 정상 import
- [ ] ANSI integer 값에 대해 warning + fallback 동작
- [ ] "none" 값 정상 처리 (fallback 대체)
- [ ] `--editor opencode --target opencode` 정규화 동작 + 정보 메시지
- [ ] `cargo test && cargo fmt --check && cargo clippy --all-targets` 통과

---

## System-Wide Impact

### Interaction Graph

- `main.rs` → `reader.rs::scan_opencode_themes()` (Editor::OpenCode) 또는 `reader.rs::ThemeReader` (VS Code/Cursor)
- `main.rs` → `converter::convert()` (VS Code) 또는 `converter_opencode::convert_opencode()` (OpenCode)
- `main.rs` → `store::save_ir()` (syntax/diff 포함된 확장 IR 저장)
- `main.rs` → `target/opencode.rs::write()` → `store::atomic_write()`
- `target/opencode.rs::post_write_action()` → tui.json 수정 제안

### Error Propagation

- OpenCode 미설치 시: `detect()` → false, 타겟 목록에서 제외 (기존 패턴)
- defs 순환 참조: 1단계만 resolve하므로 순환 자체가 불가 (by construction)
- JSON 파싱 실패: `anyhow::Result` 체인으로 에러 전파, `anyhow::bail!` 사용 (process::exit 금지)
- tui.json invalid JSON: `PostWriteAction::Guide` fallback (덮어쓰지 않음)
- ANSI integer: warn + fallback (v1)

### State Lifecycle Risks

- 기존 저장된 IR에 syntax/diff 없음 → `Option::None` + export 시 fallback 파생으로 처리
- tui.json partial write 위험 → `atomic_write` 사용 (store.rs 유틸)

### API Surface Parity

- `Target` enum: detect/write/existing_theme_path/link/post_write_action + display_name — 6개 메서드 모두 구현
- `Editor` enum: OpenCode는 `main.rs`에서 별도 경로 (detect_editors()와 분리)
- `Target::all()`: `[Target; 3]` → `[Target; 4]`

## Acceptance Criteria

### Functional Requirements

- [ ] `chromaport --editor vscode --target opencode` — VS Code 테마를 OpenCode JSON으로 변환
- [ ] `chromaport --editor cursor --target opencode` — Cursor 테마를 OpenCode JSON으로 변환
- [ ] `chromaport --editor opencode --target ghostty` — OpenCode 테마를 Ghostty로 변환
- [ ] `chromaport --editor opencode --target warp` — OpenCode 테마를 Warp으로 변환
- [ ] `chromaport --editor opencode --target superset` — OpenCode 테마를 Superset으로 변환
- [ ] `chromaport --editor opencode --target opencode` — OpenCode 테마 정규화 + 정보 메시지
- [ ] `chromaport apply` — 저장된 테마를 OpenCode 타겟으로 내보내기
- [ ] OpenCode JSON 출력이 `https://opencode.ai/theme.json` 스키마 준수
- [ ] tui.json 자동 수정 (신규 생성 / 기존 수정 / invalid JSON fallback / 수정 거부 시 가이드)
- [ ] defs 참조, ANSI integer warn+fallback, "none" 인라인 처리 정상 동작

### Non-Functional Requirements

- [ ] 기존 저장된 IR JSON 하위 호환성 유지 (serde backward compat)
- [ ] 기존 타겟(Ghostty/Warp/Superset) 동작에 영향 없음
- [ ] `cargo test` 전체 통과
- [ ] `cargo fmt --check` 통과
- [ ] `cargo clippy --all-targets` 경고 없음

### Quality Gates

- [ ] 각 Phase별 unit test 추가 — `ir::test_fixtures::make_test_ir()` 공유 fixture 사용
- [ ] legacy IR (syntax/diff None) → OpenCode export fallback 테스트
- [ ] defs 해석: 정상 참조, 미존재 키, 비-hex resolve 값 테스트
- [ ] tui.json: 존재/미존재/invalid JSON/기존 theme 4가지 케이스
- [ ] OpenCode import file size guard (1MB 초과 거부)
- [ ] OpenCode import file count guard (256개 초과 시 경고)

### Security Checklist

- [ ] `scan_opencode_themes()` — `store::is_regular_file()` 사용 (symlink 방지)
- [ ] 파일 크기 체크 (`MAX_OPENCODE_THEME_BYTES = 1MB`) 후 read_to_string
- [ ] 파일 수 체크 (`MAX_THEME_FILES = 256`) — 초과 시 warn + 무시
- [ ] defs 객체 크기 제한 (`MAX_DEFS_ENTRIES = 256`)
- [ ] defs resolve 후 HexColor::parse 검증 — 실패 시 fallback
- [ ] `$schema` URL은 메타데이터, 런타임 fetch 없음
- [ ] file permissions은 기존 `atomic_write`의 0o600 자동 적용
- [ ] `theme_slug()` 사용 필수 — 경로 순회 방지
- [ ] **기존 코드 수정**: `store::list_ir_files`의 `p.is_file()` → `store::is_regular_file()` (pre-existing 취약점)

## Dependencies & Risks

| 리스크 | 영향 | 완화 |
|---|---|---|
| OpenCode 스키마 변경 | export 포맷 깨짐 | `$schema` 참조로 버전 고정, CI에서 스키마 검증 테스트 |
| Syntax fallback 품질 | chart_colors 기반 파생이 부정확할 수 있음 | 주요 테마(One Dark, Dracula, Catppuccin)로 수동 검증 |
| tui.json 포맷 변경 | post-write 실패 | Guide fallback으로 안전하게 처리 |
| detect_editors() 반환 타입 불일치 | OpenCode가 강제로 끼워 맞춰짐 | main.rs에서 완전 별도 경로로 분리 |
| Symlink following | 임의 파일 읽기 | `store::is_regular_file()` + `theme_slug()` |

## Alternative Approaches Considered

(see brainstorm: docs/brainstorms/2026-03-13-opencode-support-brainstorm.md)

1. **타겟에서 파생**: chart_colors 5개로 syntax 9종을 추측 → 정확도 낮아 기각
2. **하이브리드 (MVP + 확장)**: 두 번 작업하게 됨 → 기각
3. **별도 Source trait 추상화**: Editor 2개 + OpenCode 1개에 trait은 과도 → Editor enum 확장 + 별도 detect으로 결정
4. **ThemeExtensions 래퍼**: syntax/diff를 wrapper struct으로 감싸기 → Technical Review에서 YAGNI 판정. 기존 `Option<T>` 패턴과 일관되게 직접 필드 추가로 변경
5. **Converter 서브모듈 디렉토리**: converter.rs를 converter/{mod,vscode,opencode}.rs로 분할 → 455줄 파일에 과도. flat sibling file로 변경
6. **handle_post_write_action 동시 추출**: feature PR에 포함 → target-agnostic이라 별도 chore: 커밋으로 분리
7. **defs export 활용**: 가독성 향상 → 프로그래매틱 생성에 불필요 (Simplicity + Performance 권장)

## Scope Summary

| 파일 | 변경 유형 | 규모 |
|---|---|---|
| `src/ir.rs` | SyntaxColors, DiffColors 구조체 + ThemeIR 필드 추가 | ~50 LOC |
| `src/converter.rs` | extract_syntax_colors() + extract_diff_colors() 추가 | ~80 LOC |
| `src/converter_opencode.rs` | **신규** — convert_opencode() | ~80 LOC |
| `src/cli.rs` | Editor::OpenCode, Target::OpenCode 추가 | ~10 LOC |
| `src/target/mod.rs` | `pub mod opencode;` + 6개 match arm + `Target::all() → [Target; 4]` | ~20 LOC |
| `src/target/opencode.rs` | **신규** — detect/write/link/post_write + format_opencode_theme | ~180 LOC |
| `src/reader.rs` | detect_opencode() + scan_opencode_themes() + resolve_color() | ~80 LOC |
| `src/main.rs` | OpenCode editor/target 분기 + 에러 메시지 | ~30 LOC |
| `src/apply.rs` | 에러 메시지 업데이트 | ~5 LOC |
| `src/store.rs` | list_ir_files의 is_file() → is_regular_file() 수정 | ~2 LOC |

**총 예상:** ~420 LOC 추가/수정 (원래 600에서 ~180 절감)

## Follow-up Work (별도 PR)

- [ ] `handle_post_write_action` 중복 추출 (`main.rs` + `apply.rs` → `target/mod.rs`로 공유) — `chore: extract shared handle_post_write_action`
- [ ] Ghostty의 로컬 `make_test_ir()` → 공유 fixture로 마이그레이션 — `chore: unify test fixtures`
- [ ] ANSI 0-15 lookup table 추가 (실제 사용 테마 발견 시) — `feat: ANSI color support for OpenCode import`
- [ ] Trait-based target dispatch 검토 (5번째 타겟 추가 시) — `refactor: target dispatch trait`

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-13-opencode-support-brainstorm.md](docs/brainstorms/2026-03-13-opencode-support-brainstorm.md) — Key decisions: ThemeIR 확장 전략, 양방향 지원, 단일 모드 출력, tui.json 자동 수정

### Internal References

- Target 패턴 참조: `src/target/ghostty.rs` (detect/write/link/post_write + `format_ghostty_theme()` 패턴)
- Store 유틸: `src/store.rs` (atomic_write, chromaport_themes_dir, theme_slug, create_symlink, `is_regular_file`)
- Color 유틸: `src/color.rs` (derive_palette, hex_from_rgb, OKLCH 변환)
- Converter 참조: `src/converter.rs:90-112` (CHART_SCOPES tokenColors 매핑)
- Test fixture: `src/ir.rs` (make_test_ir — 공유 fixture)
- **Learning**: `docs/solutions/code-quality/code-review-central-theme-store-ux-refactoring.md` — DRY, XDG 검증 통일, atomic symlink, LinkResult::Conflict, 파라미터화 프롬프트, anyhow::bail!

### External References

- OpenCode 테마 문서: https://opencode.ai/docs/themes/
- OpenCode JSON 스키마: https://opencode.ai/theme.json
- OpenCode TUI 설정: tui.json의 `theme` 필드
- Serde field attributes: https://serde.rs/field-attrs.html
