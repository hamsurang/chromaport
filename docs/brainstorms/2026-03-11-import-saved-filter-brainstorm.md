---
title: Import 시 이미 저장된 테마 마킹
type: feat
date: 2026-03-11
---

# Import 시 이미 저장된 테마 마킹

## What We're Building

기본 import 흐름(chromaport 실행)에서 TUI 테마 목록에 이미 `~/chromaport/themes/`에 IR JSON이 저장된 테마를 시각적으로 마킹하는 기능.

- 목록에서 숨기지 않고, "✔ Theme Name (saved)" 형태로 표시
- saved 테마를 다시 선택해도 정상 진행 (기존 덮어쓰기 확인 흐름 유지)
- 판단 기준: `~/chromaport/themes/{slug}.json` 파일 존재 여부

## Why This Approach

- **완전히 숨기면** 사용자가 "테마가 어디 갔지?" 혼란 발생
- **마킹만 하면** 정보를 제공하되 선택 자유를 유지
- 덮어쓰기 확인은 이미 구현되어 있으므로 재선택 시 추가 로직 불필요

## Key Decisions

1. **필터 방식**: 숨기기 X, 마킹 O — 목록에 보이되 saved 상태 표시
2. **판단 기준**: IR JSON 파일 존재 여부 (`store::list_ir_files()` 활용)
3. **재선택 동작**: 기존 흐름 유지 (덮어쓰기 확인 후 진행)
4. **마킹 위치**: TUI preview 목록의 테마 이름 옆에 표시

## Scope

- `preview/mod.rs` 또는 `preview/app.rs`에서 목록 렌더링 시 saved 상태 반영
- `store::list_ir_files()` 결과로 slug set 구성 → ThemeEntry의 settings_id와 매칭
- UI 렌더링 변경 (label에 ✔ prefix 또는 "(saved)" suffix 추가)
