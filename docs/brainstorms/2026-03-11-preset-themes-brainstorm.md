---
title: Preset/Bundled Themes
type: feat
date: 2026-03-11
---

# Preset/Bundled Themes

## What We're Building

VS Code/Cursor extension 없이도 chromaport를 사용할 수 있도록 인기 테마의 ThemeIR JSON을 미리 제공하는 기능.

- `chromaport presets install` 서브커맨드로 preset 설치
- VS Code/Cursor 미발견 시 "preset을 설치하시겠습니까?" 자동 안내
- GitHub repo의 `assets/presets/`에 IR JSON 관리, raw URL로 다운로드
- 업데이트도 고려 (새 preset 추가 시 `chromaport presets update`로 동기화)

## Why This Approach

- **include_str! 컴파일타임 포함** → 바이너리 크기 증가, 테마 추가 시 릴리즈 필요
- **별도 repo** → 관리 포인트 증가, 오버엔지니어링
- **본 repo assets/ + 서브커맨드** → 테마 추가가 PR로 가능, 바이너리 크기 영향 없음, 업데이트 가능

## Key Decisions

1. **번들링 방식**: 서브커맨드로 설치 (`chromaport presets install`)
2. **소스 위치**: chromaport repo의 `assets/presets/` 디렉토리
3. **안내 타이밍**: 에디터 미발견 시 자동 제안 + 언제든 서브커맨드로 접근
4. **초기 테마 세트**: 10개 이상 풀세트
   - One Monokai, Material Theme, Ayu Dark
   - Dracula, Catppuccin, Tokyo Night
   - Solarized Dark/Light, Gruvbox, Nord, GitHub Theme
5. **IR 생성 방법**: 개발자가 수동으로 extension 설치 → chromaport 추출 → 커밋

## Scope

- 새 서브커맨드: `Presets { Install, Update, List }`
- `assets/presets/` 디렉토리에 IR JSON 파일들
- GitHub raw URL에서 다운로드 로직 (`ureq` 이미 의존성에 있음)
- 다운로드된 preset은 `~/chromaport/themes/`에 저장 → 기존 apply 흐름과 호환
- 에디터 미발견 시 안내 메시지 (main.rs의 bail 지점 수정)

## Resolved Questions

- **preset 목록 관리**: `assets/presets/index.json` JSON manifest 파일로 관리. 클라이언트가 index만 먼저 다운로드하여 목록 표시 가능
- **원작자 attribution**: manifest에 `author`, `license`, `source_url` 필드 포함. `chromaport presets list` 출력에 표시
