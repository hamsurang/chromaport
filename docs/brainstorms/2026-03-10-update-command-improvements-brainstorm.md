# Brainstorm: update 커맨드 개선

**Date:** 2026-03-10
**Status:** Complete

## What We're Building

`chromaport update` 서브커맨드의 안정성과 UX를 개선한다.

### 현재 문제

1. **CWD 미존재 시 brew 실패**: 삭제된 디렉토리(예: 정리된 git worktree)에서 실행하면 `brew upgrade`가 `getcwd` 에러로 실패함
2. **Formula 미동기화**: GitHub release는 0.4.0이지만 Homebrew tap formula가 아직 0.3.0이면, `brew upgrade`가 "already installed"를 반환하는데 코드는 이를 성공으로 처리하고 "Updated successfully!" 출력
3. **`-v` 플래그 미지원**: `chromaport -v`가 에러 발생, `--version`만 동작

### 변경 사항

1. **`brew update` 추가**: `brew upgrade` 전에 `brew update`를 실행하여 formula를 최신 상태로 갱신
2. **확인 후 실행**: 업데이트 명령어를 보여주고 Y/n 확인 후 실행
3. **CWD 에러 메시지 개선**: brew 실패 시 CWD 문제를 감지하고 친절한 안내 메시지 출력
4. **`-v` 플래그 추가**: clap의 `short_flag = 'v'`로 `--version`의 단축 플래그 지원

## Why This Approach

- **`brew update` 추가**: formula 동기화 문제의 근본 원인 해결. `brew upgrade`만으로는 로컬 formula가 오래된 경우 업데이트 불가
- **확인 후 실행**: 사용자가 실행될 명령어를 확인할 수 있어 투명성 확보. 네트워크 작업이므로 확인이 합리적
- **에러 메시지 개선**: `$HOME`으로 자동 변경하는 대신, 사용자에게 상황을 설명하고 직접 해결하도록 안내. 예측 가능한 동작 유지
- **clap short_flag**: 가장 관용적이고 간단한 방법. 별도 인자 정의 불필요

## Key Decisions

| 결정 | 선택 | 대안 | 이유 |
|------|------|------|------|
| 서브커맨드 이름 | `update` 유지 | `update-check`로 변경 | 직접 실행 기능을 유지하므로 `update`가 적절 |
| brew 실행 방식 | `brew update` → `brew upgrade` | `brew upgrade`만 실행 | formula 동기화 문제 해결 |
| 실행 전 확인 | Y/n 프롬프트 | 바로 실행 / 안내만 | 투명성과 편의성 균형 |
| CWD 문제 대응 | 에러 메시지 개선 | `$HOME`으로 자동 변경 | 사용자에게 명시적 안내가 더 예측 가능 |
| `-v` 구현 | clap `short_flag` | 별도 `-v` 인자 | clap 관용적 패턴, 최소 코드 |

## Implementation Notes

### update 실행 플로우 (개선 후)

```
1. GitHub API로 최신 버전 확인
2. 새 버전 없으면 "이미 최신입니다" 출력 후 종료
3. 새 버전 있으면:
   a. "새 버전이 있습니다: 0.3.0 → 0.4.0" 출력
   b. 설치 방법 감지 (Homebrew/Cargo/Unknown)
   c. Homebrew인 경우:
      - "다음 명령어를 실행합니다: brew update && brew upgrade chromaport"
      - "계속하시겠습니까? (Y/n)" 프롬프트
      - 확인 시 brew update 실행 → brew upgrade chromaport 실행
      - brew 실패 시 CWD 에러 감지 → 친절한 안내 메시지
   d. Cargo인 경우: 기존과 동일 (cargo install chromaport)
   e. Unknown인 경우: 기존과 동일 (수동 안내)
```

### CWD 에러 감지

brew 실패 시 stderr에 "current working directory" 또는 "getcwd" 포함 여부로 감지하거나, 실행 전에 `std::env::current_dir()` 체크 후 선제적 안내.

### passive notification 메시지 변경

`print_update_notice()`의 "Run `chromaport update` to upgrade." 메시지는 그대로 유지 (서브커맨드명 변경 없음).

### `-v` 플래그

```rust
#[command(
    version,
    short_flag = 'v',  // 추가
    about = "..."
)]
```

## Scope

- `src/cli.rs`: `-v` 플래그 추가
- `src/update.rs`: `run_update()` 개선 (brew update 추가, Y/n 확인, CWD 에러 처리)
- `tests/cli.rs`: `-v` 플래그 테스트 추가, update 관련 테스트 업데이트
