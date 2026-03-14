# CLI UX Principles Reference

CLI 도구 설계의 핵심 원칙을 정리한 참조 문서. clig.dev, POSIX, GNU, 12 Factor CLI Apps에서 추출한 실무 적용 가능한 원칙들.

## Table of Contents
1. [Human-First Design](#1-human-first-design)
2. [Help & Documentation](#2-help--documentation)
3. [Arguments & Flags](#3-arguments--flags)
4. [Output Design](#4-output-design)
5. [Error Handling](#5-error-handling)
6. [Exit Codes](#6-exit-codes)
7. [Interactivity & TTY](#7-interactivity--tty)
8. [Configuration Hierarchy](#8-configuration-hierarchy)
9. [Composability](#9-composability)
10. [Robustness](#10-robustness)

---

## 1. Human-First Design

**출처**: clig.dev

CLI는 사람이 사용하는 도구다. 기계 파싱은 부차적이다.

- 기본 출력은 사람이 읽기 쉽게 설계한다
- 색상, 이모지, 정렬 등을 활용하여 정보 계층을 시각적으로 전달한다
- 전문 용어를 최소화하고 일상적 언어를 사용한다
- 사용자가 "다음에 뭘 해야 하지?"라고 느끼지 않게 한다

**평가 기준**: 처음 사용하는 사람이 도움말 없이 기본 작업을 수행할 수 있는가?

## 2. Help & Documentation

**출처**: clig.dev, GNU

도움말은 사용자가 가장 먼저 찾는 곳이다.

### 필수 요소
- `--help` (`-h`): 간결한 사용법, 옵션 목록, 대표 예시
- `--version` (`-V`): 프로그램명과 시맨틱 버전
- 서브커맨드별 `--help`: 각 서브커맨드의 독립적 도움말

### 도움말 구조 (권장 순서)
```
Description (한 줄 설명)
Usage (사용법 패턴)
Arguments (위치 인자)
Options (플래그와 옵션)
Subcommands (하위 명령)
Examples (실제 사용 예시 2-3개)
```

### 핵심 지침
- 예시를 반드시 포함한다. 사용자의 60%는 옵션 설명보다 예시를 먼저 본다
- `--help`는 한 화면에 들어와야 한다. 넘치면 정보를 계층화한다
- 인자 없이 실행 시 도움말 또는 유용한 안내를 보여준다 (빈 출력 금지)

**평가 기준**: `--help`만 보고 가장 흔한 3가지 작업을 수행할 수 있는가?

## 3. Arguments & Flags

**출처**: POSIX, GNU, clig.dev

일관되고 예측 가능한 인터페이스를 만든다.

### 플래그 규칙
- 짧은 플래그: `-v` (단일 문자, 하이픈 1개)
- 긴 플래그: `--verbose` (단어, 하이픈 2개)
- Boolean 플래그: `--color` / `--no-color` (부정형 제공)
- 값이 있는 옵션: `--output file.txt` 또는 `--output=file.txt`
- `--`: 이후의 모든 것을 인자로 취급 (플래그 종료)

### 인자 설계
- 필수 인자는 최소화한다 (2개 이하 권장)
- 위치 인자보다 이름 있는 플래그를 선호한다 (명시적)
- 여러 값을 받을 때: `--target a --target b` 또는 `--target a,b`

### 검증과 제안
- 잘못된 값에 대해 유효 값 목록을 보여준다
- 오타에 대해 "did you mean?" 제안을 한다
- 상충하는 옵션 조합을 사전에 감지하여 안내한다

**평가 기준**: 잘못된 인자를 넣었을 때, 에러 메시지만 보고 올바른 명령을 구성할 수 있는가?

## 4. Output Design

**출처**: clig.dev, 12 Factor CLI Apps

출력은 정보의 명확한 전달이 목적이다.

### 채널 분리
- **stdout**: 프로그램의 주요 출력 (파이프로 다른 도구에 전달 가능해야)
- **stderr**: 로그, 경고, 에러, 진행 상황 (사람을 위한 메타 정보)

이 분리가 왜 중요한가: `my-tool | jq .` 같은 파이프라인에서 에러 메시지가 stdout에 섞이면 jq가 깨진다.

### 색상 사용
- 의미를 전달하는 데 색상을 사용한다 (녹색=성공, 빨강=에러, 노랑=경고)
- 색상만으로 정보를 전달하지 않는다 (색약 사용자 고려, 아이콘 등 병용)
- `NO_COLOR` 환경변수를 존중한다 (no-color.org)
- `--no-color` 플래그를 제공한다
- 파이프로 출력될 때(`!isatty(stdout)`) 자동으로 색상을 끈다

### 출력 형식
- 기본: 사람이 읽기 좋은 형식
- `--json`: 기계 파싱용 JSON 출력 (선택적이지만 권장)
- `--quiet` (`-q`): 최소 출력 (스크립트용)
- `--verbose` (`-v`): 상세 출력 (디버깅용)

### 진행 표시
- 1초 이상 걸리는 작업에는 진행 표시를 보여준다
- 진행 표시는 stderr로 출력한다
- 가능하면 퍼센트/ETA를, 불가능하면 최소한 스피너를 보여준다
- 비-TTY에서는 진행 표시를 자동으로 끈다

**평가 기준**: 출력을 파이프로 다른 도구에 보냈을 때 문제 없이 동작하는가?

## 5. Error Handling

**출처**: clig.dev, 12 Factor CLI Apps

에러 메시지는 사용자가 문제를 해결하는 것을 돕기 위해 존재한다.

### 좋은 에러 메시지의 3요소
1. **무엇이** 잘못되었는가 (What went wrong)
2. **왜** 잘못되었는가 (Why it happened)
3. **어떻게** 고칠 수 있는가 (How to fix it)

### 예시

나쁨:
```
Error: file not found
```

좋음:
```
error: theme file not found: ~/.config/chromaport/themes/monokai.json
hint: run 'chromaport presets list' to see available themes, or 'chromaport create' to make a new one
```

### 핵심 지침
- 에러는 stderr로 출력한다
- 에러 메시지에 맥락을 포함한다 (어떤 파일, 어떤 인자, 어떤 작업 중)
- 가능하면 해결 방법을 제안한다
- 스택 트레이스는 기본으로 보여주지 않는다 (--verbose나 RUST_BACKTRACE 등으로 선택적 노출)
- 에러 형식을 일관되게 유지한다 (예: `error: 메시지\nhint: 제안`)

**평가 기준**: 에러 메시지만 보고 문제를 해결할 수 있는가?

## 6. Exit Codes

**출처**: POSIX, GNU

종료 코드는 스크립트와 자동화에서 성공/실패를 판단하는 핵심 메커니즘이다.

| 코드 | 의미 | 예시 |
|------|------|------|
| 0 | 성공 | 정상 완료, 사용자가 의도적으로 취소 |
| 1 | 일반 에러 | 실행 중 오류 발생 |
| 2 | 사용법 에러 | 잘못된 인자, 잘못된 플래그 |
| 126 | 실행 불가 | 권한 부족 |
| 127 | 명령어 없음 | 의존 프로그램 미설치 |
| 130 | Ctrl+C | SIGINT로 중단됨 |

### 핵심 지침
- 성공은 항상 0
- 사용자의 의도적 취소(Esc, 'n' 선택)도 0 (에러가 아님)
- 에러 유형에 따라 종료 코드를 구분한다
- `$?`로 확인 가능한 의미 있는 코드를 반환한다

**평가 기준**: 스크립트에서 `if my-tool; then ...`이 예상대로 동작하는가?

## 7. Interactivity & TTY

**출처**: clig.dev, 12 Factor CLI Apps

인터랙티브/비인터랙티브 환경을 모두 고려한다.

### TTY 감지
- stdin이 TTY가 아니면 (파이프 입력): 프롬프트를 건너뛰고 기본값을 사용하거나 에러를 낸다
- stdout이 TTY가 아니면 (파이프 출력): 색상, 진행 표시, 페이저를 끈다
- `--yes` (`-y`) 플래그로 확인 프롬프트를 건너뛸 수 있게 한다

### 인터랙티브 기능 설계
- 프롬프트에는 기본값을 표시한다 `[Y/n]`
- 취소(Esc)가 항상 가능하게 한다
- 현재 상태와 가능한 액션을 화면에 표시한다
- 비파괴적 기본값을 사용한다 (확신이 없으면 안전한 쪽)

### Ctrl+C 처리
- 터미널 상태를 복원한다 (raw mode, alternate screen 등)
- 임시 파일을 정리한다
- 적절한 종료 코드(130)를 반환한다

**평가 기준**: `echo "" | my-tool`이 크래시 없이 합리적으로 동작하는가?

## 8. Configuration Hierarchy

**출처**: 12 Factor CLI Apps, clig.dev

설정은 계층적으로 관리하고, 우선순위를 명확히 한다.

### 우선순위 (높은 것이 낮은 것을 override)
1. CLI 플래그 (`--config value`)
2. 환경변수 (`MY_TOOL_CONFIG=value`)
3. 프로젝트 설정 파일 (`.my-tool.toml` in CWD)
4. 사용자 설정 파일 (`~/.config/my-tool/config.toml`)
5. 시스템 설정 파일 (`/etc/my-tool/config.toml`)
6. 내장 기본값

### 핵심 지침
- 설정 파일 위치를 문서화한다
- `--config` 등으로 설정 파일 경로를 override할 수 있게 한다
- 현재 적용된 설정을 확인할 수 있는 방법을 제공한다 (`my-tool config show`)
- XDG Base Directory를 따른다 (Linux: `~/.config/`, `~/.cache/`)

**평가 기준**: 사용자가 설정 파일이 어디에 있는지, 어떤 값이 적용되고 있는지 알 수 있는가?

## 9. Composability

**출처**: POSIX, clig.dev

다른 도구와 조합하여 사용할 수 있게 설계한다.

### 핵심 지침
- stdin에서 입력을 받을 수 있게 한다 (가능한 경우)
- stdout 출력을 파이프로 보낼 수 있게 한다
- 한 가지 일을 잘 한다 (Unix 철학)
- 부작용(side effect)을 명시적으로 표시한다

### 스크립트 친화성
- 비인터랙티브 모드를 지원한다 (모든 입력을 플래그로 제공 가능)
- 결정적(deterministic) 동작을 보장한다 (같은 입력 → 같은 출력)
- `--quiet`로 스크립트에서 불필요한 출력을 억제할 수 있게 한다

**평가 기준**: 셸 스크립트에서 이 도구를 사용하여 자동화할 수 있는가?

## 10. Robustness

**출처**: 12 Factor CLI Apps, clig.dev

예상치 못한 상황에서도 우아하게 동작한다.

### 네트워크
- 적절한 타임아웃을 설정한다 (무한 대기 금지)
- 오프라인 상태에서도 가능한 기능은 동작해야 한다
- 네트워크 에러 시 무엇이 실패했고 왜인지 알려준다
- 재시도 가능한 에러는 재시도 방법을 안내한다

### 파일 시스템
- 원자적(atomic) 쓰기를 사용한다 (temp + rename)
- 기존 파일을 덮어쓰기 전에 확인한다
- 설정 변경 전에 백업을 만든다
- 필요한 디렉토리를 자동으로 생성한다

### 상태 관리
- 빈 상태(첫 사용)에서 도움이 되는 메시지를 보여준다
- 손상된 상태에서 복구 방법을 안내한다
- 동시 실행 시 데이터 충돌을 방지한다

### 시그널 처리
- SIGINT (Ctrl+C): 깔끔하게 종료, 상태 복원
- SIGTERM: 가능하면 정리 후 종료
- SIGPIPE: 조용히 종료 (파이프가 닫혀도 에러 출력 안 함)

**평가 기준**: 예상치 못한 상황(네트워크 끊김, 디스크 꽉 참, Ctrl+C)에서 사용자 데이터가 안전한가?
