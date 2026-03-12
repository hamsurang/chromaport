# TUI Screenshot Automation for README

**Date**: 2026-03-11
**Status**: Brainstorm

## What We're Building

VHS(Charmbracelet)를 사용하여 chromaport TUI의 인터랙티브 테마 프리뷰 워크플로우를 GIF 애니메이션으로 자동 캡처하고, 이를 README 상단(로고 바로 아래)에 메인 데모로 배치한다.

### 범위

- VHS `.tape` 스크립트 작성: chromaport TUI의 테마 탐색 → 필터링 → 선택 → 라이브 프리뷰 흐름
- GIF를 `assets/` 폴더에 저장
- README.md 업데이트: 로고 아래에 GIF 삽입

### 범위 외

- CI/CD 자동화 (필요 시 나중에 추가)
- Makefile/justfile 통합
- 정적 PNG 스크린샷
- 다른 워크플로우(에디터/타겟 선택 CLI 부분) 별도 캡처

## Why This Approach

### VHS를 선택한 이유

1. **재현성**: `.tape` 파일은 선언적 스크립트로, 동일한 결과를 반복 생성 가능
2. **TUI 친화적**: ratatui/crossterm 기반 TUI의 키보드 인터랙션을 자연스럽게 시뮬레이션
3. **간단한 사용법**: `vhs demo.tape` 한 줄로 GIF 생성
4. **생태계**: Charmbracelet의 널리 사용되는 도구로, 잘 관리되고 문서화됨

### GIF를 선택한 이유

- TUI의 인터랙티브한 특성(테마 탐색, 필터링, 라이브 프리뷰 전환)을 정적 이미지로는 전달 불가
- GitHub README에서 바로 재생 가능 (외부 링크 불필요)

### 로고 아래 배치를 선택한 이유

- 첫 방문자에게 chromaport의 핵심 가치를 즉시 시각적으로 전달
- "Your favorite editor theme, everywhere" 태그라인의 실체를 바로 보여줌

## Key Decisions

1. **도구**: VHS (Charmbracelet) — `.tape` 스크립트 기반 터미널 GIF 레코딩
2. **결과물 형태**: GIF 애니메이션 (단일 파일, 전체 프리뷰 흐름)
3. **캡처 워크플로우**: 테마 리스트 탐색 → 필터링 → 테마 선택 → 라이브 프리뷰
4. **저장 위치**: `assets/demo.gif` (또는 `assets/chromaport-demo.gif`)
5. **README 배치**: 로고 이미지 바로 아래, `# chromaport` 제목 위 또는 태그라인 아래
6. **자동화 수준**: 수동 실행 (`vhs demo.tape`) — YAGNI 원칙 적용

## Implementation Considerations

### VHS .tape 스크립트 구성 요소

```tape
# 터미널 설정
Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"  # 또는 적절한 터미널 테마

# chromaport 실행 (에디터/타겟은 CLI 인자로 미리 지정)
Type "chromaport --editor cursor --target ghostty"
Enter
Sleep 2s

# TUI 인터랙션: 테마 탐색
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms

# 필터링
Type "cat"  # "Catppuccin" 등 필터
Sleep 1s

# 테마 선택
Enter
Sleep 2s
```

### 주의 사항

- chromaport 바이너리가 PATH에 있어야 함 (또는 `cargo run --` 사용)
- VS Code/Cursor 테마 확장이 설치된 환경에서만 실행 가능
- VHS 설치 필요: `brew install vhs`
- GIF 파일 크기 관리 필요 (README 로딩 속도에 영향)

## Open Questions

_현재 열린 질문 없음 — 모든 주요 결정이 대화에서 해결됨._
