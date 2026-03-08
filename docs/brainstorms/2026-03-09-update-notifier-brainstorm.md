# Brainstorm: CLI Update Notifier

**Date:** 2026-03-09
**Status:** Draft

## What We're Building

chromaport 실행 시 새 버전이 출시되었는지 확인하고, 업데이트가 있으면 실행 완료 후 하단에 안내 메시지를 표시하는 기능. gh CLI, yarn, oh-my-zsh 등이 제공하는 것과 동일한 패턴.

**예시 출력:**
```
  ✔ One Monokai → ~/.superset/app-state.json

A new release of chromaport is available: 0.2.0 → 0.3.0
To update, run: brew upgrade chromaport
https://github.com/hamsurang/chromaport/releases/tag/v0.3.0
```

## Why This Approach

### GitHub Releases API

- chromaport는 이미 GitHub Releases + cargo-dist로 배포 중
- `https://api.github.com/repos/hamsurang/chromaport/releases/latest` 한 번의 GET 요청으로 최신 버전 조회 가능
- crates.io보다 Homebrew 릴리스와 동기화가 정확함
- 인증 불필요 (public repo)

### 24시간 주기 체크

- 매 실행마다 네트워크 요청을 보내지 않아 성능 영향 최소화
- 로컬 캐시 파일에 마지막 체크 타임스탬프 + 최신 버전 저장
- 캐시 위치: `~/.chromaport/update-check.json` 또는 `dirs::cache_dir()`
- gh CLI가 동일한 패턴 사용

### 실행 끝 하단 표시

- 핵심 작업 흐름을 방해하지 않음
- 사용자가 작업 결과를 먼저 확인한 뒤 업데이트 안내를 봄
- stderr가 아닌 stdout으로 출력 (인터랙티브 CLI이므로 파이프라인 우려 낮음)

### 오프라인/실패 시 조용히 스킵

- 업데이트 체크는 부가 기능이므로 핵심 기능을 절대 방해하지 않음
- 타임아웃 짧게 설정 (2-3초)
- 네트워크 오류 시 에러 메시지 없이 정상 진행

## Key Decisions

| 결정 | 선택 | 근거 |
|------|------|------|
| 버전 소스 | GitHub Releases API | 배포 파이프라인과 일치, 인증 불필요 |
| 체크 주기 | 24시간 캐시 | 네트워크 부하 최소화, gh CLI 패턴 |
| 표시 위치 | 실행 완료 후 하단 | 작업 흐름 비침해 |
| 오프라인 동작 | 조용히 스킵 | 부가 기능이 핵심 기능 방해 금지 |
| HTTP 클라이언트 | ureq | 경량, blocking, async 런타임 불필요 |
| 캐시 위치 | `dirs::cache_dir()/chromaport/` | XDG 규격 준수, OS 관례 따름 |
| 자체 업데이트 | `chromaport update` 서브커맨드 | 설치 경로 감지 후 적절한 명령어 대신 실행 |

## Implementation Sketch (High-Level)

1. **캐시 파일 관리** — 마지막 체크 시각 + 최신 버전을 `dirs::cache_dir()/chromaport/update-check.json`에 저장
2. **GitHub API 호출** — `releases/latest` 엔드포인트에서 `tag_name` 파싱 (ureq, 타임아웃 3초)
3. **버전 비교** — semver 비교로 현재 < 최신이면 알림 플래그 설정
4. **알림 출력** — main 작업 완료 후 조건부로 업데이트 안내 메시지 출력
5. **`chromaport update` 서브커맨드** — 설치 경로 감지(Homebrew prefix / cargo) 후 적절한 업데이트 명령어 실행
6. **비활성화 옵션** — `CHROMAPORT_NO_UPDATE_CHECK=1` 환경 변수로 끄기

### `chromaport update` 동작

```
$ chromaport update
Checking for updates...
Updating chromaport 0.2.0 → 0.3.0
✔ Updated successfully!
```

설치 경로 감지 전략:
- Homebrew: `brew --prefix chromaport` 성공 여부로 판단 → `brew upgrade chromaport` 실행
- Cargo: `cargo install --list | grep chromaport` 확인 → `cargo install chromaport` 실행
- 감지 실패 시: 수동 업데이트 안내 + 릴리즈 URL 표시

## Resolved Questions

- **HTTP 클라이언트**: `ureq` 선택. 경량 blocking 클라이언트로 기존 동기 CLI와 자연스럽게 맞음.
- **캐시 위치**: `dirs::cache_dir()/chromaport/` 선택. XDG 규격 준수, OS 관례 따름.
- **업데이트 방식**: `chromaport update` 자체 업데이트 서브커맨드 제공. 설치 경로를 자동 감지하여 적절한 명령어(brew upgrade / cargo install)를 대신 실행.
