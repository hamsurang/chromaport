---
title: "fix: Superset theme import fails on null optional fields"
type: fix
status: completed
date: 2026-03-10
origin: docs/brainstorms/2026-03-10-fix-superset-theme-import-validation-brainstorm.md
---

# fix: Superset theme import fails on null optional fields

## Overview

chromaport가 생성한 Superset 테마 JSON을 import하면 "Theme 1: Invalid input" 에러 발생. Rust `Option::None`이 JSON `null`로 직렬화되는데, Superset의 Zod schema `z.string().optional()`은 `null`을 허용하지 않음 (`undefined`/absent만 허용).

## Problem Statement

(see brainstorm: docs/brainstorms/2026-03-10-fix-superset-theme-import-validation-brainstorm.md)

생성된 JSON:
```json
{
  "terminal": {
    "cursorAccent": null,              // ← z.string().optional() 실패
    "selectionBackground": "#80CBC420" // ← Some일 때는 정상
  }
}
```

Superset import schema (`shared/themes/import.ts`):
```typescript
cursorAccent: z.string().optional(),      // string | undefined, NOT null
selectionBackground: z.string().optional(),
```

## Proposed Solution

### Phase 1: Fix `ir_to_json` in `src/target/superset.rs`

`serde_json::json!` 매크로는 조건부 필드 생략이 불가하므로, terminal 객체를 `serde_json::Map`으로 동적 구성:

```rust
// src/target/superset.rs — ir_to_json 함수 내 terminal 블록
let mut terminal = serde_json::Map::new();
// 필수 필드: 항상 포함
terminal.insert("background".into(), json!(t.background.as_str()));
terminal.insert("foreground".into(), json!(t.foreground.as_str()));
terminal.insert("cursor".into(), json!(t.cursor.as_str()));
// ... 16개 ANSI 색상 필드 ...

// Optional 필드: Some일 때만 포함, None이면 생략
if let Some(ref c) = t.cursor_accent {
    terminal.insert("cursorAccent".into(), json!(c.as_str()));
}
if let Some(ref c) = t.selection_bg {
    terminal.insert("selectionBackground".into(), json!(c.as_str()));
}
```

최종 JSON 조립 시 `"terminal": serde_json::Value::Object(terminal)` 사용.

### Phase 2: Test Case 보강

현재 테스트 현황 (`src/target/superset.rs`):
- `ir_to_json_contains_required_fields` — top-level 필드 검증
- `ir_to_json_ui_colors_mapped` — UI 색상 매핑
- `ir_to_json_terminal_colors_mapped` — terminal 색상 (make_test_ir의 기본값만)

**추가할 테스트:**

#### 2-1. `ir_to_json_omits_none_terminal_fields`
`cursor_accent: None`, `selection_bg: None`일 때 JSON에 해당 키가 **아예 없는지** 검증.

```rust
// src/target/superset.rs
#[test]
fn ir_to_json_omits_none_terminal_fields() {
    let mut ir = make_test_ir();
    ir.terminal.cursor_accent = None;
    ir.terminal.selection_bg = None;
    let json = ir_to_json(&ir);
    assert!(json["terminal"].get("cursorAccent").is_none());
    assert!(json["terminal"].get("selectionBackground").is_none());
}
```

#### 2-2. `ir_to_json_includes_some_terminal_optional_fields`
`cursor_accent: Some(...)`, `selection_bg: Some(...)`일 때 값이 정상 포함되는지 검증.

```rust
#[test]
fn ir_to_json_includes_some_terminal_optional_fields() {
    let mut ir = make_test_ir();
    let c = |s: &str| HexColor::parse(s).unwrap();
    ir.terminal.cursor_accent = Some(c("#FF0000"));
    ir.terminal.selection_bg = Some(c("#00FF00"));
    let json = ir_to_json(&ir);
    assert_eq!(json["terminal"]["cursorAccent"], "#FF0000");
    assert_eq!(json["terminal"]["selectionBackground"], "#00FF00");
}
```

#### 2-3. `ir_to_json_no_null_values`
생성된 JSON 전체에서 `null` 값이 없는지 재귀적 검증 (regression guard).

```rust
#[test]
fn ir_to_json_no_null_values() {
    let ir = make_test_ir();
    let json = ir_to_json(&ir);
    fn assert_no_nulls(value: &serde_json::Value, path: &str) {
        match value {
            serde_json::Value::Null => panic!("null found at {}", path),
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    assert_no_nulls(v, &format!("{}.{}", path, k));
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    assert_no_nulls(v, &format!("{}[{}]", path, i));
                }
            }
            _ => {}
        }
    }
    assert_no_nulls(&json, "root");
}
```

#### 2-4. `ir_to_json_terminal_has_all_required_ansi_colors`
terminal 블록이 16개 ANSI 색상 + background/foreground/cursor를 빠짐없이 포함하는지 검증.

```rust
#[test]
fn ir_to_json_terminal_has_all_required_ansi_colors() {
    let ir = make_test_ir();
    let json = ir_to_json(&ir);
    let terminal = &json["terminal"];
    let required = [
        "background", "foreground", "cursor",
        "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
        "brightBlack", "brightRed", "brightGreen", "brightYellow",
        "brightBlue", "brightMagenta", "brightCyan", "brightWhite",
    ];
    for key in required {
        assert!(
            terminal.get(key).is_some(),
            "missing required terminal key: {}", key
        );
    }
}
```

## Acceptance Criteria

- [x] `ir_to_json`에서 `cursor_accent: None`일 때 JSON에 `cursorAccent` 키 absent
- [x] `ir_to_json`에서 `selection_bg: None`일 때 JSON에 `selectionBackground` 키 absent
- [x] `cursor_accent: Some(...)`, `selection_bg: Some(...)`일 때 정상 포함
- [x] 생성된 JSON에 `null` 값이 없음 (regression guard)
- [x] terminal 블록에 19개 필수 필드 모두 포함
- [x] 기존 테스트 3개 통과 유지
- [x] 새 테스트 4개 추가 및 통과
- [x] `cargo test && cargo fmt --check && cargo clippy --all-targets` 클린
- [x] Superset에서 실제 import 성공 확인 (수동)

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-10-fix-superset-theme-import-validation-brainstorm.md](../brainstorms/2026-03-10-fix-superset-theme-import-validation-brainstorm.md)
- Superset import schema: `~/Desktop/projects/superset/apps/desktop/src/shared/themes/import.ts`
- chromaport target: `src/target/superset.rs:51-123` (`ir_to_json`)
- IR types: `src/ir.rs` (`AnsiColors.cursor_accent`, `AnsiColors.selection_bg`)
