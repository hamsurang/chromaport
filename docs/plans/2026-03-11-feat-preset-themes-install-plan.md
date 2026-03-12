---
title: "feat: Add preset themes install command"
type: feat
status: active
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-preset-themes-brainstorm.md
---

# feat: Add preset themes install command

## Overview

VS Code/Cursor extension 없이도 chromaport를 사용할 수 있도록 인기 테마의 ThemeIR JSON을 GitHub repo에서 다운로드하여 설치하는 `chromaport presets` 서브커맨드 제공.

## Problem Statement / Motivation

현재 chromaport는 VS Code/Cursor extension이 설치되어 있어야만 동작한다. extension 없이 Superset/Warp/Ghostty만 사용하는 유저는 chromaport를 쓸 수 없다. 인기 테마를 미리 추출해두면 이 진입 장벽을 제거할 수 있다.

## Proposed Solution

(see brainstorm: docs/brainstorms/2026-03-11-preset-themes-brainstorm.md)

1. `assets/presets/index.json` — manifest with theme metadata (name, slug, author, license, source_url)
2. `assets/presets/{slug}.json` — 각 테마의 ThemeIR JSON
3. `chromaport presets install` — manifest 다운로드 → 테마 선택 → `~/chromaport/themes/`에 저장
4. `chromaport presets list` — 설치 가능한 preset 목록 표시 (installed 마킹)
5. 에디터 미발견 시 "preset을 설치하시겠습니까?" 자동 안내

> `presets update`는 MVP에서 제외. `install`이 매번 최신 manifest를 다운받으므로 동일한 역할을 수행. 필요 시 추후 추가.

## Technical Considerations

- **HTTP**: `ureq` 이미 의존성에 있음. `update.rs`의 `Agent` 패턴 재활용 (3초 timeout)
- **GitHub raw URL**: `https://raw.githubusercontent.com/hamsurang/chromaport/main/assets/presets/{file}`
- **저장 위치**: `~/chromaport/themes/{slug}.json` — 기존 apply 흐름과 완전 호환
- **Manifest 캐싱**: 불필요 (presets install은 명시적 사용자 액션)
- **Attribution**: manifest에 author/license 포함, `presets list`에서 표시

## Acceptance Criteria

- [ ] `chromaport presets list` — 설치 가능한 preset 목록 + 이미 설치된 건 "(installed)" 마킹
- [ ] `chromaport presets install` — 인터랙티브 MultiSelect로 테마 선택 → 다운로드 → `~/chromaport/themes/`에 저장
- [ ] 에디터 미발견 시 preset 설치 안내 메시지 출력
- [ ] 10개 이상 인기 테마 IR JSON 포함
- [ ] 네트워크 오류 시 graceful 에러 메시지
- [ ] 각 preset에 원작자 attribution (author, license) 표시

## Implementation

### Phase 1: Manifest 및 IR JSON 준비

개발자가 수동으로 각 테마의 VS Code extension을 설치 → chromaport로 추출 → IR JSON을 `assets/presets/`에 커밋.

**`assets/presets/index.json` 구조:**

```json
{
  "version": 1,
  "themes": [
    {
      "slug": "one-monokai",
      "name": "One Monokai",
      "author": "azemoh",
      "license": "MIT",
      "source_url": "https://marketplace.visualstudio.com/items?itemName=azemoh.one-monokai"
    }
  ]
}
```

**초기 테마 목록 (10+):**

| Theme | Author |
|-------|--------|
| One Monokai | azemoh |
| Material Theme | Mattia Astorino |
| Ayu Dark | teabyii |
| Dracula | Dracula Theme |
| Catppuccin Mocha | Catppuccin |
| Tokyo Night | enkia |
| Solarized Dark | Ryan Olson |
| Solarized Light | Ryan Olson |
| Gruvbox Dark | jdinhify |
| Nord | arcticicestudio |
| GitHub Dark | GitHub |

### Phase 2: CLI 서브커맨드 추가

**`src/cli.rs` 변경:**

```rust
#[derive(Subcommand)]
pub enum Command {
    Update { #[arg(short = 'y', long)] yes: bool },
    Apply,
    /// Manage preset themes
    Presets {
        #[command(subcommand)]
        action: PresetsAction,
    },
}

#[derive(Subcommand)]
pub enum PresetsAction {
    /// List available preset themes
    List,
    /// Install preset themes
    Install,
}
```

### Phase 3: Presets 모듈 구현 (`src/presets.rs`)

```rust
// src/presets.rs

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/hamsurang/chromaport/main/assets/presets/index.json";
const THEME_BASE_URL: &str = "https://raw.githubusercontent.com/hamsurang/chromaport/main/assets/presets";

pub fn run_list() -> Result<()>     // manifest 다운 → 목록 출력 (installed 마킹)
pub fn run_install() -> Result<()>  // manifest 다운 → MultiSelect → 다운로드 → 저장
```

**핵심 흐름 (`run_install`):**
1. `fetch_manifest()` — ureq로 index.json 다운로드 → `Vec<PresetEntry>` 파싱
2. `store::list_ir_files()` — 이미 설치된 slug set 구성 (파일명에서 stem 추출)
3. 미설치 테마만 필터링
4. `inquire::MultiSelect` — presets.rs 내에서 직접 사용 (별도 interactive 함수 불필요)
5. 선택된 각 테마: `fetch_theme_json(slug)` → `serde_json::from_str::<ThemeIR>()` → `store::save_ir(&ir)`
6. 결과 출력

**HTTP 패턴** (`update.rs` 동일):
```rust
let config = ureq::Agent::config_builder()
    .timeout_global(Some(Duration::from_secs(10)))
    .build();
let agent: ureq::Agent = config.into();
let body: String = agent.get(url).call()?.body_mut().read_to_string()?;
```

### Phase 4: 에디터 미발견 시 안내 (`src/main.rs`)

```rust
// src/main.rs — 기존 all_editors.is_empty() bail 부분 수정
if all_editors.is_empty() {
    eprintln!("No VS Code or Cursor installation found.");
    eprintln!("Run `chromaport presets install` to use preset themes instead.");
    std::process::exit(1);
}
```

### 변경 파일 목록

| 파일 | 변경 내용 |
|------|----------|
| `assets/presets/index.json` | 신규 — manifest |
| `assets/presets/{slug}.json` | 신규 — 10+ 테마 IR JSON |
| `src/cli.rs` | `Presets` 서브커맨드 + `PresetsAction` enum |
| `src/presets.rs` | 신규 — presets 모듈 (list/install) |
| `src/main.rs` | `mod presets` + dispatch + 에디터 미발견 안내 수정 |

## Dependencies & Risks

- **네트워크 의존**: GitHub raw URL 접근 불가 시 presets 기능 동작 불가. 에러 메시지로 안내
- **라이선스 준수**: 각 테마 원작자 attribution 필수. manifest에 source_url/license 포함
- **GitHub rate limit**: raw.githubusercontent.com은 rate limit이 느슨하지만, 대량 다운로드 시 주의
- **IR 호환성**: 향후 ThemeIR 구조 변경 시 preset JSON 업데이트 필요. manifest `version` 필드로 대응

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-preset-themes-brainstorm.md](docs/brainstorms/2026-03-11-preset-themes-brainstorm.md) — 서브커맨드 설치 방식, GitHub assets/ 소스, 10+ 풀세트, manifest 관리, attribution 결정
- HTTP 패턴: `src/update.rs:134-137`
- IR 저장: `src/store.rs:185` (`save_ir()`)
- MultiSelect 패턴: `src/interactive.rs:97` (`select_targets_multi()`)
