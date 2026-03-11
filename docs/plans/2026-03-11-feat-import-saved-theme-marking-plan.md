---
title: "feat: Mark saved themes in import TUI list"
type: feat
status: active
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-import-saved-filter-brainstorm.md
---

# feat: Mark saved themes in import TUI list

Import TUI 목록에서 이미 `~/chromaport/themes/`에 IR JSON이 저장된 테마를 "(saved)" 마킹으로 표시한다.

## Acceptance Criteria

- [ ] TUI 테마 목록에서 저장된 테마 옆에 `(saved)` suffix 표시
- [ ] saved 테마 선택 시 기존 흐름 동일 (덮어쓰기 확인 후 진행)
- [ ] saved 판단 기준: `~/chromaport/themes/{slug}.json` 존재 여부
- [ ] 필터 검색 시에도 saved 마킹 유지
- [ ] 기존 `[active]` 마킹과 공존 (e.g. `Theme Name [active] (saved)`)
- [ ] 성능: saved slug set 구성은 TUI 진입 시 1회만 수행

## Implementation

### Phase 1: Saved slugs 수집 (`src/preview/mod.rs`)

`select_theme_with_preview()` 진입부에서 saved slug set 구성:

```rust
// src/preview/mod.rs — select_theme_with_preview() 내부, PreviewApp 생성 전
use std::collections::HashSet;
use crate::store;

let saved_slugs: HashSet<String> = store::list_ir_files()
    .unwrap_or_default()
    .iter()
    .filter_map(|p| p.file_stem()?.to_str().map(|s| s.to_string()))
    .collect();

let mut app = PreviewApp::new(sorted, active_id.map(str::to_string), reader, target, saved_slugs);
```

### Phase 2: PreviewApp에 saved_slugs 전달 (`src/preview/app.rs`)

```rust
// PreviewApp 구조체에 필드 추가
saved_slugs: HashSet<String>,

// new()에 파라미터 추가
pub fn new(..., saved_slugs: HashSet<String>) -> Self { ... }

// saved 여부를 filtered indices 기준으로 반환 (UI에 직접 전달)
pub fn filtered_saved_flags(&self) -> Vec<bool> {
    self.filtered_indices
        .iter()
        .map(|&i| {
            let slug = store::theme_slug(&self.all_themes[i].label);
            self.saved_slugs.contains(&slug)
        })
        .collect()
}
```

> **설계 근거**: slug 매칭은 `app.rs`에서 수행하여 `ui.rs`가 `store`에 직접 의존하지 않도록 한다. `converter::convert()`에서 `ir.name = entry.label.clone()`이므로 `theme_slug(label)`과 saved IR 파일명이 일치함을 보장.

### Phase 3: UI 렌더링 변경 (`src/preview/ui.rs`)

`render_theme_list()`에 `saved_flags` 파라미터 추가, `[active]` 패턴과 동일하게 처리:

```rust
// src/preview/ui.rs:227 — render_theme_list 시그니처 변경
pub fn render_theme_list(
    f: &mut Frame,
    area: Rect,
    labels: &[String],
    selected: usize,
    filter: &str,
    active_id: Option<&str>,
    settings_ids: &[String],
    saved_flags: &[bool],  // 추가
)

// src/preview/ui.rs:265 — display 구성 부분 변경
let is_saved = saved_flags.get(i).copied().unwrap_or(false);
let display = match (is_active, is_saved) {
    (true, true) => format!("{label} [active] (saved)"),
    (true, false) => format!("{label} [active]"),
    (false, true) => format!("{label} (saved)"),
    (false, false) => label.clone(),
};
```

### Phase 4: 호출부 업데이트

- `src/preview/mod.rs:90` — `ui::render_theme_list()` 호출에 `app.filtered_saved_flags()` 전달
- `src/preview/ui.rs` 테스트 — `render_theme_list` 호출에 빈 `&[]` 전달

### 변경 파일 목록

| 파일 | 변경 내용 |
|------|----------|
| `src/preview/mod.rs` | saved_slugs 구성 + PreviewApp 생성 시 전달 + render 호출 시 전달 |
| `src/preview/app.rs` | `saved_slugs` 필드/파라미터/getter 추가 |
| `src/preview/ui.rs` | `render_theme_list()` 시그니처에 `saved_slugs` 추가 + display 로직 |

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-import-saved-filter-brainstorm.md](docs/brainstorms/2026-03-11-import-saved-filter-brainstorm.md) — 마킹 방식, IR JSON 판단 기준, 재선택 동작 결정
- 기존 `[active]` 패턴: `src/preview/ui.rs:262-269`
- Slug 생성: `src/store.rs:78` (`theme_slug()`)
- IR 목록: `src/store.rs:206` (`list_ir_files()`)
