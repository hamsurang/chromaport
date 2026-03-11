# Brainstorm: `chromaport apply` Command

**Date**: 2026-03-11
**Status**: Complete

## What We're Building

`chromaport apply` — 이미 import하여 저장된 ThemeIR(chromaport 스키마)를 다른 target에 re-import 없이 바로 적용하는 인터랙티브 서브커맨드.

### 사용자 플로우

```
$ chromaport apply
```

1. `~/chromaport/themes/` 루트에 저장된 IR 파일(.json) 목록을 TUI 프리뷰로 표시
2. 사용자가 theme 하나를 선택
3. 해당 theme이 아직 export되지 않은 target만 복수 선택(multi-select) 가능하게 표시
4. 선택한 target들에 대해 변환 및 export 수행

### 전제 조건: IR 자동 저장

기존 import 플로우(기본 `chromaport` 명령어)에서 target export와 함께 ThemeIR도 `~/chromaport/themes/{slug}.json`으로 자동 저장하도록 수정 필요.

## Why This Approach

### 순수 인터랙티브 (선택된 접근법)

- CLI 플래그 없이 `chromaport apply`만으로 동작
- 기존 import 플로우와 깔끔하게 분리된 별도 서브커맨드
- YAGNI 원칙에 충실 — 스크립팅 지원은 필요할 때 추가

### 기각된 접근법

- **하이브리드 (인터랙티브 + CLI 플래그)**: 유연하지만 현시점에서 스크립팅 수요 불명확
- **메인 플로우 통합**: 기존 플로우 복잡도 증가, 의도 분리 불명확

## Key Decisions

| 결정 사항 | 선택 | 이유 |
|-----------|------|------|
| IR 저장 위치 | `~/chromaport/themes/{slug}.json` (루트) | target 폴더와 같은 레벨에 나란히, 직관적 |
| IR 저장 시점 | 기존 import 플로우에서 자동 저장 | 기존 사용자도 자연스럽게 apply 사용 가능 |
| target 필터링 | 미적용 target만 표시 (파일 존재 여부로 판단) | 불필요한 선택지 제거, 깔끔한 UX |
| 재import 시 IR | 무조건 덮어쓰기 | 항상 최신 상태 유지, 단순함 |
| theme 선택 UI | ratatui TUI 프리뷰 | 기존 프리뷰 인프라 재활용, 풍부한 UX |
| target 선택 | 복수 선택(multi-select) | 여러 target에 한 번에 적용 가능, 효율적 |
| CLI 인터페이스 | 순수 인터랙티브 (플래그 없음) | YAGNI, 단순함 우선 |

## 디렉토리 구조

```
~/chromaport/themes/
├── one-dark-pro.json      # ThemeIR (chromaport schema)
├── material-theme.json    # ThemeIR
├── superset/              # target exports
│   └── chromaport-one-dark-pro.json
├── warp/
│   └── one-dark-pro.yaml
└── ghostty/
    └── One Dark Pro
```

## 구현 범위

### In Scope

1. **IR 자동 저장**: 기존 import 플로우에서 ThemeIR을 `~/chromaport/themes/{slug}.json`으로 저장
2. **`chromaport apply` 서브커맨드**: IR 목록 → TUI 프리뷰 → theme 선택 → 미적용 target multi-select → export
3. **미적용 target 감지**: 각 target 디렉토리에 해당 slug의 파일이 존재하는지 확인

### Out of Scope (향후 별도 브레인스토밍)

- import 시 이미 저장된 theme 필터링
- 빌트인 프리셋 테마 (One Monokai, Material Theme 등)
- `chromaport create` 명령어 (색상 팔레트 기반 테마 생성)
- CLI 플래그를 통한 non-interactive apply

## Edge Cases

- 저장된 IR이 하나도 없을 때: 안내 메시지 ("No saved themes. Run `chromaport` to import first.")
- 선택한 theme이 모든 설치된 target에 이미 적용됨: 안내 메시지 표시 후 다른 theme 선택 유도
- 설치된 target이 하나도 없을 때 (Warp/Ghostty/Superset 미설치): 에러 메시지와 함께 종료
- IR 파일이 손상되었거나 역직렬화 실패 시: 해당 파일 건너뛰고 경고 표시
- 미적용 판단 기준: target 디렉토리에 해당 slug의 파일이 존재하면 "적용됨"으로 간주 (내용 비교 안 함)
