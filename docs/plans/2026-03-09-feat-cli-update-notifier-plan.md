---
title: "feat: Add CLI update notifier and self-update command"
type: feat
status: completed
date: 2026-03-09
origin: docs/brainstorms/2026-03-09-update-notifier-brainstorm.md
---

# feat: Add CLI update notifier and self-update command

## Overview

chromaport 실행 시 새 버전이 출시되었는지 확인하고, 업데이트가 있으면 실행 완료 후 하단에 안내 메시지를 표시하는 기능을 추가한다. 또한 `chromaport update` 서브커맨드를 통해 설치 경로를 자동 감지하여 적절한 업데이트 명령어를 대신 실행하는 자체 업데이트 기능을 제공한다.

gh CLI, yarn, oh-my-zsh 등 널리 사용되는 CLI 도구가 제공하는 패턴과 동일하다.

## Problem Statement

현재 chromaport 사용자는 새 버전이 출시되어도 직접 GitHub Releases 페이지를 방문하거나 `brew outdated` / `cargo install --list` 등을 실행하지 않는 한 알 수 없다. 이로 인해:

- 버그 수정이나 새 타겟 지원 등 중요한 업데이트를 놓침
- 사용자가 오래된 버전에서 겪는 문제를 이미 수정된 버전에서 보고하는 경우 발생
- 수동으로 업데이트 여부를 확인해야 하는 번거로움

## Proposed Solution

두 가지 핵심 기능을 제공한다:

### 1. Passive Update Notice (자동 알림)

일반 `chromaport` 실행 시 백그라운드에서 버전을 체크하고, 새 버전이 있으면 실행 완료 후 하단에 한 줄 안내를 표시한다.

```
  ✔ One Monokai → ~/.superset/app-state.json

A new release of chromaport is available: 0.2.0 → 0.3.0
Run `chromaport update` to upgrade.
https://github.com/hamsurang/chromaport/releases/tag/v0.3.0
```

### 2. Active Update Command (자체 업데이트)

`chromaport update` 서브커맨드로 설치 경로를 자동 감지하여 적절한 업데이트 명령어를 대신 실행한다.

```
$ chromaport update
Checking for updates...
Updating chromaport 0.2.0 → 0.3.0
✔ Updated successfully!
```

## Technical Approach

### Architecture

기존 flat `Parser` 구조에 optional `Subcommand`를 추가하여, 서브커맨드 없이 실행하면 기존 동작(테마 마이그레이션)을 유지하고, `update` 서브커맨드가 주어지면 업데이트 플로우를 실행한다.

새로운 `src/update.rs` 모듈에 모든 업데이트 관련 로직을 캡슐화한다.

```
src/
  cli.rs         ← Subcommand enum 추가
  main.rs        ← subcommand 분기 + 실행 후 알림 출력
  update.rs      ← NEW: 업데이트 체크, 캐시, 알림, 자체 업데이트 로직
```

### Implementation Phases

#### Phase 1: Passive Update Notification — CLI 구조 + 캐시 + API + 알림

`cli.rs`에 optional subcommand를 추가하고, `update.rs`에 캐시, GitHub API 연동, 버전 비교, 알림 출력을 모두 구현한다.

**Tasks:**

- [x] `src/cli.rs`: `Command` enum 추가 (`#[derive(Subcommand)]`), `Cli` struct에 `#[command(subcommand)] pub command: Option<Command>` 필드 추가
- [x] `src/main.rs`: `cli.command` 매치 — `Some(Command::Update)` → 업데이트 플로우, `None` → 기존 동작
- [x] `src/update.rs`: 새 모듈 생성
- [x] `src/update.rs`: 캐시 스키마 정의 (`UpdateCache` struct with serde) + `read_cache()` / `write_cache()`
  - `write_cache()`는 `store::atomic_write` 재활용
- [x] `src/update.rs`: `is_cache_fresh()` — 7일 TTL 체크
- [x] `src/update.rs`: `fetch_latest_version()` — ureq로 GitHub API GET 요청
  - URL: `https://api.github.com/repos/hamsurang/chromaport/releases/latest`
  - User-Agent: `chromaport/{version}`
  - Accept: `application/vnd.github+json`
  - 타임아웃: 3초 (connect + read)
- [x] `src/update.rs`: GitHub API 응답에서 `tag_name` 파싱, `v` 접두사 제거, `semver::Version` 비교
- [x] `src/update.rs`: `check_for_update()` — 비활성화 체크 → 캐시 체크 → 필요시 API 호출 → 캐시 갱신 → 결과 반환
  - 비활성화 조건: `CHROMAPORT_NO_UPDATE_CHECK=1`, `CI` 환경변수, non-TTY (`std::io::stderr().is_terminal()`)
- [x] `src/update.rs`: `print_update_notice()` — stderr로 알림 메시지 출력
- [x] `src/main.rs`: `run()` 성공 후 `check_for_update()` 호출, 결과가 있으면 `print_update_notice()` 호출
- [x] 에러 처리: 네트워크 실패, non-200 응답, JSON 파싱 실패, 캐시 쓰기 실패 — 모두 `Ok(None)` 반환 (조용히 스킵)
- [x] `Cargo.toml`: `ureq = "3"`, `semver = "1"` 의존성 추가
- [x] `tests/cli.rs`: 기존 테스트가 서브커맨드 추가 후에도 통과하는지 확인

**캐시 파일 스키마:**

```json
{
  "last_checked_at": "2026-03-09T12:00:00Z",
  "latest_version": "0.3.0"
}
```

- `last_checked_at`: ISO 8601 UTC 타임스탬프
- `latest_version`: GitHub에서 조회한 최신 버전 (v 접두사 제거)

**캐시 위치:**

| OS    | 경로                                          |
|-------|-----------------------------------------------|
| macOS | `~/Library/Caches/chromaport/update-check.json` |
| Linux | `~/.cache/chromaport/update-check.json`         |

**알림 메시지 형식 (stderr):**

```
A new release of chromaport is available: 0.2.0 → 0.3.0
Run `chromaport update` to upgrade.
https://github.com/hamsurang/chromaport/releases/tag/v0.3.0
```

**비활성화 우선순위:**

1. `CHROMAPORT_NO_UPDATE_CHECK=1` → 스킵
2. `CI` 환경변수 감지 (`std::env::var("CI").is_ok()`) → 스킵
3. non-TTY (`!std::io::stderr().is_terminal()`) → 스킵
4. 캐시 유효 (7일 이내) → 캐시된 결과 사용
5. 그 외 → GitHub API 호출

**참고:** `--version`, `--help`는 clap이 `run()` 전에 조기 종료하므로 업데이트 체크가 자연스럽게 실행되지 않음.

**성공 기준:** `chromaport --help`에 `update` 서브커맨드 표시, 기존 플래그 동작 유지, 캐시 만료 시 GitHub API 호출, 신규 버전 존재 시 알림 출력, 네트워크 실패 시 무시, CI에서 자동 비활성화.

**예상 변경 파일:** `src/cli.rs`, `src/main.rs`, `src/update.rs` (new), `Cargo.toml`

#### Phase 2: `chromaport update` 서브커맨드 구현

설치 경로를 자동 감지하여 적절한 업데이트 명령어를 실행하는 자체 업데이트 기능을 구현한다.

**Tasks:**

- [x] `src/update.rs`: `detect_install_method()` — `std::env::current_exe()` canonical path 기반 감지
  - Homebrew: 경로에 `/Cellar/` 또는 `/homebrew/` 포함 여부
  - Cargo: 경로에 `/.cargo/bin/` 포함 여부
  - 감지 실패: `Unknown` 반환
- [x] `src/update.rs`: `run_update()` — 자체 업데이트 실행
  - GitHub API 강제 호출 (캐시 무시, `CHROMAPORT_NO_UPDATE_CHECK` 무시)
  - 버전 비교 → 이미 최신이면 안내 후 종료
  - 설치 경로 감지 → `std::process::Command`로 적절한 명령어 실행
  - 자식 프로세스 stdout/stderr inherit, exit code 전파
- [x] `src/main.rs`: `Some(Command::Update)` 분기에서 `run_update()` 호출

**실행 명령어:**

| 감지 결과   | 명령어                      |
|-------------|----------------------------|
| Homebrew    | `brew upgrade chromaport`  |
| Cargo       | `cargo install chromaport` |
| Unknown     | 수동 안내 + 릴리즈 URL     |

**출력 예시:**

```
# 성공 시
$ chromaport update
Checking for updates...
Updating chromaport 0.2.0 → 0.3.0 via Homebrew...
[brew upgrade 출력이 그대로 표시됨]
✔ Updated successfully!

# 네트워크 실패 시
$ chromaport update
Checking for updates...
Error: Could not check for updates: connection timed out
```

**Exit codes:**

| 시나리오             | Exit code |
|---------------------|-----------|
| 이미 최신            | 0         |
| 업데이트 성공        | 0         |
| 네트워크 실패        | 1         |
| 업그레이드 명령 실패  | 자식 프로세스 exit code 전파 |

**성공 기준:** 설치 경로 자동 감지, 적절한 업그레이드 명령어 실행, 네트워크 실패 시 에러 메시지 출력.

**예상 변경 파일:** `src/update.rs`, `src/main.rs`

#### Phase 3: 테스트 + 문서화 + 마무리

단위 테스트, 통합 테스트를 추가하고 README를 업데이트한다.

**Tasks:**

- [x] `src/update.rs`: 단위 테스트
  - `test_parse_github_tag_name` — `v0.3.0` → `0.3.0` 파싱
  - `test_compare_versions` — 다양한 버전 비교 케이스
  - `test_cache_read_write` — 캐시 파일 읽기/쓰기/만료 체크
  - `test_cache_missing_or_corrupted` — 누락/손상된 캐시 처리
  - `test_detect_install_method` — 경로 기반 설치 방법 감지
- [x] `tests/cli.rs`: 통합 테스트
  - `test_update_subcommand_help` — `chromaport update --help` 출력 확인
  - `test_existing_flags_still_work` — 기존 `--editor`, `--target` 플래그 동작 확인
  - `test_help_shows_update_subcommand` — `--help`에 `update` 서브커맨드 표시 확인
  - **기존 플래그 호환성**: `chromaport --editor vscode --target warp --yes` — 서브커맨드 추가 후에도 동일 동작
  - **CI 환경**: `CI=true` 설정 → 업데이트 체크 전체 스킵
- [x] `README.md`: Update 섹션에 `chromaport update` 사용법 추가
- [x] `README.md`: Options 섹션에 `update` 서브커맨드 반영
- [x] `README.md`: `CHROMAPORT_NO_UPDATE_CHECK` 환경변수 문서화
- [x] `cargo fmt-check && cargo lint && cargo test` 전체 통과 확인

**성공 기준:** 모든 테스트 통과, clippy 경고 0, 기존 테스트 퇴행 없음.

**예상 변경 파일:** `src/update.rs`, `tests/cli.rs`, `README.md`

## Alternative Approaches Considered

### 1. crates.io API 사용

crates.io 레지스트리에서 최신 버전을 조회하는 방식. Cargo로 설치한 사용자에게는 자연스럽지만, Homebrew 사용자에게는 crates.io 배포 시점과 Homebrew formula 업데이트 시점이 다를 수 있어 혼란을 줄 수 있다. GitHub Releases가 배포 파이프라인(cargo-dist)과 직접 연결되어 있으므로 더 정확하다. (see brainstorm: docs/brainstorms/2026-03-09-update-notifier-brainstorm.md)

### 2. 백그라운드 스레드를 통한 비동기 체크

`std::thread::spawn`으로 HTTP 호출을 병렬 실행하여 메인 작업과 동시에 수행하는 방식. 체감 지연이 없지만 구현 복잡도가 높아진다. chromaport는 자주 실행하는 도구가 아니고 7일 캐시 TTL로 실제 네트워크 호출 빈도가 낮으므로, 동기 호출의 3초 최대 지연은 수용 가능하다. (see brainstorm)

### 3. 안내만 표시하고 자체 업데이트 미지원

업데이트 알림만 표시하고 `chromaport update` 서브커맨드 없이 수동 업데이트를 안내하는 방식. 구현이 단순하지만 사용자 경험이 떨어진다. gh CLI, oh-my-zsh 등이 자체 업데이트를 지원하는 것이 사용자에게 더 친절하다. (see brainstorm)

## Acceptance Criteria

### Functional Requirements

- [ ] `chromaport` 일반 실행 시, 7일 캐시 만료 후 GitHub Releases API를 호출하여 최신 버전 확인
- [ ] 신규 버전 존재 시 실행 완료 후 stderr에 업데이트 알림 표시
- [ ] `chromaport update` 서브커맨드로 최신 버전 확인 및 자동 업데이트 실행
- [ ] 설치 경로(Homebrew/Cargo) 자동 감지하여 적절한 업그레이드 명령어 실행
- [ ] 감지 실패 시 수동 업데이트 안내 + 릴리즈 URL 표시
- [ ] `CHROMAPORT_NO_UPDATE_CHECK=1` 환경변수로 passive 체크 비활성화 (explicit `update` 명령에는 미적용)
- [ ] CI 환경(`CI=true`) 및 non-TTY에서 passive 체크 자동 비활성화
- [ ] 네트워크 실패, API 에러, 캐시 손상 시 passive 체크는 조용히 스킵
- [ ] `chromaport update` 네트워크 실패 시 에러 메시지 출력 + exit 1
- [ ] `chromaport --version`, `chromaport --help` 실행 시 업데이트 체크 미실행

### Non-Functional Requirements

- [ ] HTTP 타임아웃 3초 이내
- [ ] 캐시 유효 시 추가 지연 0ms (파일 읽기만)
- [ ] 새 의존성: `ureq`, `semver` — 바이너리 크기 증가 최소화
- [ ] clippy 경고 0, `cargo fmt-check` 통과

### Quality Gates

- [ ] 단위 테스트: 캐시, 버전 비교, 설치 경로 감지
- [ ] 통합 테스트: 기존 CLI 호환성, `update` 서브커맨드
- [ ] `cargo test && cargo fmt-check && cargo lint` 전체 통과

## Success Metrics

- 사용자가 새 버전 출시 후 7일 이내에 업데이트 안내를 확인할 수 있음
- `chromaport update` 한 번의 명령으로 최신 버전으로 업데이트 가능
- 기존 사용자의 스크립트나 워크플로우에 영향 없음 (하위 호환성 완전 유지)

## Dependencies & Prerequisites

### 새 크레이트 의존성

| 크레이트 | 버전 | 용도 | 크기 영향 |
|---------|------|------|----------|
| `ureq`  | 3    | 경량 blocking HTTP 클라이언트 | ~200KB (rustls 포함) |
| `semver`| 1    | 시맨틱 버전 파싱/비교 | ~50KB |

### 외부 의존성

- GitHub Releases API (`api.github.com`) — public, 인증 불필요, rate limit 60 req/hour/IP
- `releases/latest` 엔드포인트는 pre-release를 자동 필터링 (의도한 동작)

## Risk Analysis & Mitigation

| 리스크 | 확률 | 영향 | 완화 전략 |
|--------|------|------|----------|
| GitHub API rate limiting (60/hr) | 낮음 (7일 캐시) | 낮음 | 조용히 스킵, 다음 실행에서 재시도 |
| 네트워크 불안정 | 중간 | 없음 | 3초 타임아웃 + 조용히 스킵 |
| 바이너리 크기 증가 | 확실 | 낮음 | ureq는 경량, 수백KB 수준 |
| 기존 CLI 호환성 깨짐 | 낮음 | 높음 | Optional subcommand으로 기존 플래그 유지 + 통합 테스트 |
| self-replace during cargo install | 낮음 (Unix) | 없음 | Unix에서 안전 (old inode 유지) |
| 양쪽 설치 감지 충돌 (brew+cargo) | 낮음 | 중간 | current_exe 경로 기준 판단 |
| 기업 프록시 환경 | 중간 | 낮음 | v1 scope 외, 문서에 제한 사항 기재 |

## Future Considerations

- **HTTP 프록시 지원**: `HTTPS_PROXY` 환경변수를 통한 프록시 지원 (v2)
- **릴리즈 노트 미리보기**: `chromaport update --changelog`로 변경 사항 확인 (v2)
- **`chromaport update --dry-run`**: 실제 실행 없이 어떤 명령이 실행될지 미리보기 (v2)
- **추가 설치 경로 감지**: Nix, Scoop (Windows) 등 (타겟 플랫폼 확장 시)

## Documentation Plan

- [ ] `README.md`: Update 섹션 — `chromaport update` 사용법
- [ ] `README.md`: Options 섹션 — `update` 서브커맨드 반영
- [ ] `README.md`: 환경변수 섹션 — `CHROMAPORT_NO_UPDATE_CHECK` 문서화
- [ ] CLI help text: `chromaport update --help` 설명

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-09-update-notifier-brainstorm.md](../brainstorms/2026-03-09-update-notifier-brainstorm.md)
  - Key decisions: GitHub Releases API 사용, ureq HTTP 클라이언트, `chromaport update` 자체 업데이트 서브커맨드, 오프라인 시 조용히 스킵

### Internal References

- CLI 정의: `src/cli.rs:1-43` — 현재 flat Parser 구조
- 엔트리포인트: `src/main.rs:1-186` — run() 함수 7단계 파이프라인
- Atomic write: `src/store.rs` — `atomic_write()` 함수 재활용 가능
- Console 출력 스타일: `src/target/mod.rs` — `console::Style` 사용 패턴
- dirs 사용: `src/reader.rs` — `dirs::home_dir()` 패턴

### External References

- GitHub Releases API: `GET /repos/{owner}/{repo}/releases/latest`
- ureq crate: https://crates.io/crates/ureq
- semver crate: https://crates.io/crates/semver
- gh CLI update-notifier 참고: https://github.com/cli/cli

### Related Work

- cargo-dist 배포 설정: `Cargo.toml` `[workspace.metadata.dist]`
- Homebrew tap: `hamsurang/homebrew-chromaport`
