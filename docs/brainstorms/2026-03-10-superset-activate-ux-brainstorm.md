# Brainstorm: 테마 적용 UX 개편 — 중앙 저장소 + 터미널별 워크플로우

**Date:** 2026-03-10
**Status:** Final

## What We're Building

chromaport의 테마 저장/적용 아키텍처를 전면 개편한다.

1. **중앙 테마 저장소** (`~/.chromaport/themes/`) 도입 — 모든 export 테마를 한 곳에서 관리
2. **`--activate`, `--no-activate`, `--yes` 플래그 제거** — 터미널별 최적의 워크플로우로 대체
3. **Ghostty**: 인터랙티브 프롬프트 + symlink
4. **Superset**: 테마 파일 export + UI import 가이드 (직접 수정 포기)
5. **Warp**: symlink 방식으로 전환

### 핵심 문제

1. **Superset `--activate` 실패**: `activeThemeId`를 JSON에 직접 써도 Zustand persist + tRPC 레이어가 이를 무시 → UI 미반영
2. **Superset 프로세스 감지 불가**: 꺼도 프로세스가 살아있어 사용자가 종료하기 어려움
3. **`customThemes` 직접 쓰기도 불안정**: 완전 종료가 아닌 이상 `app-state.json` 수정이 덮어씌워짐
4. **`--activate`가 Ghostty에서만 유효**: 3개 터미널 중 1개만 지원하는 글로벌 플래그는 UX 혼란

## Why This Approach

각 터미널의 상태 관리 방식이 근본적으로 다르므로, 하나의 `--activate` 플래그로 통합하는 것은 불가능하다.

- **Superset**: lowdb + Zustand가 외부 수정을 무시 → 파일 export + UI import이 유일한 안정적 경로
- **Ghostty**: config 파일 수정이 안정적 → 인터랙티브 프롬프트가 플래그보다 직관적
- **Warp**: 테마 디렉토리에 파일만 놓으면 자동 인식 → symlink으로 충분

중앙 저장소는 "내가 export한 테마 어디 있지?"를 해결하고, 향후 테마 관리(목록, 삭제 등) 기능의 기반이 된다.

## Key Decisions

### 1. 중앙 테마 저장소 도입

```
~/.chromaport/themes/
├── ghostty/
│   └── One Dark Pro              # Ghostty text config 포맷
├── warp/
│   └── one-dark-pro.yaml         # Warp YAML 포맷
└── superset/
    └── chromaport-one-dark-pro.json  # Superset Theme JSON 포맷
```

- 모든 export 테마의 원본이 이 디렉토리에 저장됨
- 터미널별 서브디렉토리로 포맷 구분
- Ghostty/Warp는 해당 터미널의 테마 디렉토리에 **symlink** 생성
- `~/.chromaport/` 경로는 macOS 우선 설계. Linux XDG 준수는 이번 스코프 밖.

**Symlink 구조:**
```
~/.config/ghostty/themes/One Dark Pro
  → ~/.chromaport/themes/ghostty/One Dark Pro

~/.warp/themes/one-dark-pro.yaml
  → ~/.chromaport/themes/warp/one-dark-pro.yaml
```

### 2. CLI 플래그 정리

| 플래그 | 변경 | 비고 |
|--------|------|------|
| `--activate` | **제거** | |
| `--no-activate` (deprecated) | **제거** | |
| `--yes` | **제거** | 비TTY 환경 지원 드롭 (breaking change, 0.x 내 minor bump) |
| `--output` | **추가하지 않음** | 중앙 경로 고정 |

**Breaking change 참고:** `--yes` 제거로 비인터랙티브 사용 경로가 제거됨. chromaport는 인터랙티브 전용 CLI로 포지셔닝. 0.x 버전이므로 1.0 bump 없이 minor 버전만 올림.

### 3. Ghostty: 인터랙티브 프롬프트

```
✔ One Dark Pro → ~/.chromaport/themes/ghostty/One Dark Pro
  Linked → ~/.config/ghostty/themes/One Dark Pro

Apply to Ghostty config? (y/N): y

✔ Backed up config → config.bak.1710000000
✔ Updated config: theme = One Dark Pro
  Reload Ghostty config (Cmd+Shift+,) to apply.
```

- 테마 파일을 `~/.chromaport/themes/ghostty/`에 쓰고, `~/.config/ghostty/themes/`에 symlink 생성
- "Apply to config?" 프롬프트 → Yes: `config.bak.{timestamp}` 백업 + config 수정, No: 가이드만
- 비인터랙티브 환경: 프롬프트 스킵, 가이드만 출력
- CI/스크립트용 자동 적용 플래그는 추가하지 않음

**구현 참고 — Ghostty 경로 분리:**
- **Symlink 대상**: `ghostty_xdg_dir()`로 결정 (`~/.config/ghostty/themes/`). Ghostty는 XDG 디렉토리에서만 커스텀 테마를 인식함.
- **Config 파일**: `ghostty_config_dir()`로 결정 (macOS: `~/Library/Application Support/com.mitchellh.ghostty/config`). 이 두 경로는 다르므로 구현 시 반드시 구분할 것.

### 4. Superset: 테마 파일 export + 가이드

```
✔ chromaport-one-dark-pro.json → ~/.chromaport/themes/superset/chromaport-one-dark-pro.json

  Open Superset → Settings → Appearance →
  Import Theme → select the file above.
```

- `~/.chromaport/themes/superset/`에 Superset Theme JSON 형식으로 저장
- `app-state.json` 직접 수정 **완전 제거** (activate, customThemes write 모두)
- 파일명: `chromaport-{theme-slug}.json` (chromaport가 만든 파일임을 명시)
- Symlink 없음 (Superset은 file picker로 import하므로 symlink 불필요)

**구현 참고:** `superset::write()`에서 `is_superset_running()` 가드를 제거할 것. 더 이상 `app-state.json`을 건드리지 않으므로 Superset 실행 여부와 무관.

### 5. Warp: symlink 방식으로 전환

```
✔ one-dark-pro.yaml → ~/.chromaport/themes/warp/one-dark-pro.yaml
  Linked → ~/.warp/themes/one-dark-pro.yaml

  Open Warp → Settings → Appearance → Themes to select it.
```

- 원본을 `~/.chromaport/themes/warp/`에 저장
- `~/.warp/themes/`에 symlink 생성 (기존: 직접 쓰기)
- 가이드 메시지는 현재와 동일

### 6. 파일 충돌 처리

**중앙 저장소 파일 덮어쓰기:**
같은 테마를 다시 export할 때 → **"이미 존재합니다. 덮어쓸까요? (y/N)"** 확인 프롬프트

**Symlink 충돌 (Ghostty/Warp):**
symlink 대상 경로에 이미 일반 파일이 존재할 때 → **"파일이 이미 존재합니다. symlink로 대체할까요? (y/N)"** 확인 프롬프트. 기존 symlink이 있으면 조용히 재생성. Broken symlink도 자동 교체.

### 7. 아키텍처 변경 — write + post-write 분리

현재 `main.rs`의 write(step 6) → activate(step 7) 구조를 다음으로 변경:

1. **write**: 중앙 저장소에 테마 파일 저장
2. **link**: symlink 생성 (Ghostty/Warp만 해당)
3. **post-write**: 터미널별 후속 동작
   - Ghostty: config 수정 프롬프트
   - Superset: import 가이드 출력
   - Warp: 가이드 출력

`run_activate()` 함수는 제거하고, Target trait에 `link()`, `post_write()` 메서드를 추가하는 방향.

## Changes Summary

| 항목 | Before | After |
|------|--------|-------|
| 테마 저장 위치 | 각 터미널 디렉토리에 직접 | `~/.chromaport/themes/{target}/` (중앙) |
| `--activate` | 글로벌 플래그 | **제거** |
| `--no-activate` | deprecated 플래그 | **제거** |
| `--yes` | activate 확인 + 비TTY 테마 선택 | **제거** (breaking change) |
| Superset write | `app-state.json` 직접 수정 | 독립 JSON 파일 export |
| Superset activate | `activeThemeId` 변경 | **제거** (UI import 가이드) |
| Superset 실행 감지 | `is_superset_running()` 가드 | **제거** (불필요) |
| Ghostty write | 터미널 디렉토리에 직접 | 중앙 저장소 + symlink |
| Ghostty activate | `--activate` 플래그 필요 | 인터랙티브 프롬프트 (y/N) |
| Ghostty backup | `config.chromaport-backup` | `config.bak.{timestamp}` |
| Warp write | 터미널 디렉토리에 직접 | 중앙 저장소 + symlink |
| Warp 가이드 | 변경 없음 | 변경 없음 |
| Target trait | `write()` + `activate()` | `write()` + `link()` + `post_write()` |

## Resolved Questions

1. **`--output` 필요 여부** → 불필요. 중앙 경로 `~/.chromaport/themes/` 고정.
2. **테마 파일명 규칙** → `chromaport-{theme-slug}.json` (Superset), 다른 터미널은 기존 네이밍 유지.
3. **비인터랙티브 시 config 자동 수정** → 불필요. chromaport는 인터랙티브 CLI이므로 CI/스크립트 지원 불필요.
4. **`--output` 타겟 범위** → 해당 없음 (플래그 자체를 추가하지 않음).
5. **덮어쓰기 처리** → 확인 프롬프트 "이미 존재합니다. 덮어쓸까요? (y/N)"
6. **`--yes` 제거** → 제거. 비TTY 지원 드롭은 breaking change이나 0.x이므로 minor bump만.
7. **Symlink 충돌** → 일반 파일 존재 시 경고 + 대체 프롬프트. 기존/broken symlink은 자동 교체.
8. **Ghostty 경로 분리** → themes는 XDG dir, config는 config dir. 구현 시 반드시 구분.
9. **Superset 실행 감지** → `is_superset_running()` 가드 제거. `app-state.json`을 더 이상 건드리지 않으므로 불필요.
10. **아키텍처** → Target trait에 `link()` + `post_write()` 추가, `activate()` + `run_activate()` 제거.
