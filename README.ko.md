<p align="center">
  <img src="assets/chromaport.png" alt="Chromaport" width="600" />
</p>

# chromaport

> 내가 좋아하는 에디터 테마, 어디서든.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

[English](./README.md)

---

## 이름

**chroma** (색상) + **port** (옮기다) — 에디터 색상을 어디든 옮겨줍니다.

---

## 설치

### Homebrew

```sh
brew tap hamsurang/chromaport
brew install chromaport
```

### Cargo

```sh
cargo install chromaport
```

### 소스에서 빌드

```sh
git clone https://github.com/hamsurang/chromaport.git
cd chromaport
cargo install --path .
```

---

## 업데이트

chromaport는 일주일에 한 번 새 릴리스를 자동으로 확인합니다. 업데이트가 있으면 명령어 실행 후 안내가 표시됩니다:

```
A new release of chromaport is available: 0.2.0 → 0.3.0
Run `chromaport update` to upgrade.
```

업데이트하려면:

```sh
chromaport update
```

설치 방식(Homebrew 또는 Cargo)을 자동 감지하여 적절한 업그레이드 명령을 실행합니다.

자동 업데이트 확인을 끄려면:

```sh
export CHROMAPORT_NO_UPDATE_CHECK=1
```

CI 환경과 비대화형 셸에서는 자동으로 비활성화됩니다.

---

## 사용법

`chromaport`를 실행하고 대화형 프롬프트를 따라가세요:

```
$ chromaport
> Select editor: Cursor
> Select themes to migrate: One Monokai, Ayu Dark
> Select target app: Superset

Converting 2 theme(s)...
  ✔ One Monokai → ~/.config/chromaport/themes/one-monokai.json
  ✔ Ayu Dark → ~/.config/chromaport/themes/ayu-dark.json
```

### 명령어

```
chromaport [OPTIONS] [COMMAND]

Commands:
  update   업데이트 확인 및 chromaport 업그레이드
  apply    저장된 테마를 다른 대상에 적용
  create   처음부터 커스텀 테마 만들기
  presets  프리셋 테마 관리

Options:
  -v, --version          버전 출력
  -e, --editor <EDITOR>  소스 에디터 [가능한 값: vscode, cursor]
  -t, --target <TARGET>  대상 앱 [가능한 값: superset, warp, ghostty]
  -h, --help             도움말 출력
```

### 테마 가져오기

```sh
# 대화형 모드 — 에디터, 테마, 대상을 단계별로 선택
chromaport

# 비대화형 — 에디터와 대상을 직접 지정
chromaport --editor vscode --target ghostty
```

테마 선택 시 ratatui 기반 **TUI 라이브 미리보기**가 제공되어, 탐색하면서 각 테마를 실시간으로 확인할 수 있습니다. 방향키로 이동하고 타이핑으로 필터링하세요.

### 저장된 테마 적용

```sh
chromaport apply
```

이전에 가져온 테마를 다른 대상에 다시 적용합니다. 이미 적용된 대상도 함께 표시됩니다.

### 커스텀 테마 만들기

```sh
chromaport create
```

**대화형 색상 피커**로 테마를 처음부터 만들 수 있습니다:

1. 배경 색상 선택
2. 전경 색상 선택
3. 강조 색상 선택
4. 미리보기 후 확인

색상 피커는 **슬라이더 모드**(방향키로 HSL 값 조정, Shift로 5배 단위 이동)와 **헥스 모드**(`#`을 눌러 헥스 코드 직접 입력)를 지원합니다. 3가지 기본 색상에서 전체 팔레트가 자동으로 생성됩니다.

### 프리셋 테마

```sh
# 사용 가능한 프리셋 목록
chromaport presets list

# 프리셋 설치
chromaport presets install
```

chromaport 저장소에서 엄선된 프리셋 테마를 탐색하고 설치할 수 있습니다.

---

## 지원 에디터

| 에디터  | 경로                    |
| ------- | ----------------------- |
| VS Code | `~/.vscode/extensions/` |
| Cursor  | `~/.cursor/extensions/` |

## 지원 대상

| 대상     | 동작 방식                                                                         |
| -------- | --------------------------------------------------------------------------------- |
| Superset | `~/.superset/chromaport-themes/`에 기록 — Superset UI에서 가져오기               |
| Warp     | `~/.warp/themes/`에 심볼릭 링크 — 실행 중 자동 감지                              |
| Ghostty  | `~/.config/ghostty/themes/`에 심볼릭 링크 — 설정 파일 또는 리로드로 적용         |

---

## 동작 원리

1. 에디터 확장 디렉토리에서 `package.json`의 테마 기여(contribution)를 스캔
2. VS Code 테마 JSON 파싱 (JSONC 주석 제거 및 `include` 상속 처리)
3. 중간 표현(IR)으로 변환
4. 중앙 테마 저장소(`~/.config/chromaport/themes/`)에 저장
5. 선택한 대상 형식으로 심볼릭 링크 또는 기록

---

## 개발

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets
```

---

## 라이선스

MIT — [LICENSE](./LICENSE) 참고
