# Brainstorm: chromaport 테마 저장 경로 변경

**Date:** 2026-03-11
**Status:** Decided

## What We're Building

`.chromaport` 디렉토리명에서 `.` 접두사를 제거하여 `~/chromaport/`로 변경한다.

## Problem

Superset은 테마를 import할 때 macOS 파일 업로드 UI(NSOpenPanel)를 사용한다. macOS에서 `.`으로 시작하는 폴더는 기본적으로 숨겨져 있어, 사용자가 `Cmd+Shift+.`을 눌러야만 `.chromaport` 폴더를 볼 수 있다. 이는 직관적이지 않은 UX 문제다.

## Why This Approach

### 검토한 옵션

| 옵션 | 장점 | 단점 |
|------|------|------|
| **`~/chromaport/` (선택)** | 최소 변경, 문제 직접 해결 | 홈 디렉토리에 폴더 노출 |
| `~/Documents/chromaport/` | 사용자 친화적 위치 | iCloud 동기화 이슈 가능 |
| `~/Library/Application Support/` | macOS 표준 경로 | Library도 숨김 폴더라 문제 미해결 |

### 선택 이유

- **변경 범위 최소**: `src/store.rs`의 `chromaport_themes_dir()` 함수에서 `".chromaport"` → `"chromaport"` 한 줄 변경
- **문제 직접 해결**: macOS 파일 다이얼로그에서 즉시 보임
- **YAGNI**: 복잡한 플랫폼별 경로 로직 불필요

## Key Decisions

1. **경로**: `~/.chromaport/` → `~/chromaport/`
2. **마이그레이션 없음**: 기존 `~/.chromaport/` 사용자는 직접 복사해야 함. 자동 마이그레이션은 하지 않음
3. **구조 유지**: `~/chromaport/themes/{target}/` 하위 구조는 그대로 유지

## Scope

- `src/store.rs`: `chromaport_themes_dir()` 경로 문자열 변경
- `src/store.rs`: atomic symlink temp 파일명 패턴 확인 (`.chromaport_tmp_` 접두사도 변경 필요 여부)
- 테스트 업데이트
- README/문서 업데이트 (해당 시)
