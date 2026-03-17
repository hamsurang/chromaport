# Brainstorm: iTerm2 Color Scheme Support

**Date:** 2026-03-17
**Status:** Complete
**Participants:** User, Claude

## What We're Building

Chromaport에 iTerm2를 입력 소스와 출력 타겟 양쪽으로 추가한다.

- **입력**: iTerm2 Preferences plist(`~/Library/Preferences/com.googlecode.iterm2.plist`)의 `Custom Color Presets`에서 사용자가 저장한 커스텀 컬러 프리셋을 자동 스캔하여 ThemeIR로 변환
- **출력**: VSCode/Cursor 테마를 .itermcolors (XML plist) 포맷으로 변환하여 `~/chromaport/themes/iterm2/`에 생성, Import 방법 안내

## Why This Approach

### plist crate 통합 사용을 선택한 이유

- iTerm2 Preferences 파일이 바이너리 plist 포맷이므로 `plist` crate가 필수
- 출력(.itermcolors)도 XML plist 포맷이므로 같은 crate로 일관되게 처리
- 단일 의존성 추가로 입출력 모두 해결

### 로컬 파일만 지원하는 이유

- iterm2colorschemes.com은 참고용, 네트워크 의존성 없이 단순하게 유지
- 사용자가 이미 iTerm2에 저장한 프리셋을 활용하는 것이 자연스러운 워크플로우
- YAGNI: 다운로드 기능은 필요 시 추후 추가 가능

## Key Decisions

### 1. 입력: Preferences plist에서 Custom Color Presets 자동 스캔

- `~/Library/Preferences/com.googlecode.iterm2.plist` 파싱
- `Custom Color Presets` 딕셔너리에서 각 프리셋 추출
- float RGB (0.0~1.0) → HexColor 변환: `round(component * 255)`
- iTerm2가 설치되어 있고 커스텀 프리셋이 있을 때만 표시

### 2. 출력: .itermcolors XML plist 생성

- `~/chromaport/themes/iterm2/{slug}.itermcolors`에 파일 생성
- `plist::to_writer_xml()` 로 메모리에 생성 후 `store::atomic_write()` 로 파일에 쓰기 (crash-safe)

### 3. 설치: 파일 생성 + Import 안내

- `~/chromaport/themes/iterm2/{slug}.itermcolors` 파일만 생성 (심링크 없음)
- iTerm2에서 Import하는 방법 안내 출력: `Preferences > Profiles > Colors > Color Presets > Import`
- .itermcolors 파일은 더블클릭으로도 iTerm2에 직접 import 가능

### 4. 의존성: `plist` crate 추가

- Cargo.toml에 `plist = "1"` 추가
- serde feature 활용하여 타입 안전한 역직렬화 가능
- 바이너리/XML plist 모두 자동 감지

### 5. 색상 매핑

#### iTerm2 → ThemeIR (입력)

| iTerm2 Key | ThemeIR 필드 |
|---|---|
| `Ansi 0 Color` ~ `Ansi 7 Color` | `terminal.normal.black` ~ `terminal.normal.white` |
| `Ansi 8 Color` ~ `Ansi 15 Color` | `terminal.bright.black` ~ `terminal.bright.white` |
| `Background Color` | `terminal.background`, `background` |
| `Foreground Color` | `terminal.foreground`, `foreground` |
| `Cursor Color` | `terminal.cursor`, `cursor` |
| `Selection Color` | `terminal.selection_bg`, `selection_bg` |

#### ThemeIR → iTerm2 (출력)

| ThemeIR 필드 | iTerm2 Key | 비고 |
|---|---|---|
| `terminal.normal.black` ~ `white` | `Ansi 0 Color` ~ `Ansi 7 Color` | 직접 매핑 |
| `terminal.bright.black` ~ `white` | `Ansi 8 Color` ~ `Ansi 15 Color` | 직접 매핑 |
| `terminal.background` | `Background Color` | 직접 매핑 |
| `terminal.foreground` | `Foreground Color` | 직접 매핑 |
| `terminal.cursor` | `Cursor Color` | 직접 매핑 |
| `selection_bg` | `Selection Color` | 직접 매핑 |
| `foreground` | `Selected Text Color` | 파생 |
| `background` | `Cursor Text Color` | 파생 |
| `foreground` | `Bold Color` | 파생: foreground 그대로 |
| `accent` | `Link Color` | 파생: accent 색상 |

`Badge Color`, `Underline Color`, `Tab Color`, `Cursor Guide Color`는 생략 — iTerm2가 자체 기본값 사용.

## .itermcolors 출력 예시

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Ansi 0 Color</key>
    <dict>
        <key>Alpha Component</key>
        <real>1</real>
        <key>Blue Component</key>
        <real>0.20392</real>
        <key>Color Space</key>
        <string>sRGB</string>
        <key>Green Component</key>
        <real>0.17255</real>
        <key>Red Component</key>
        <real>0.15686</real>
    </dict>
    <!-- ... 나머지 색상들 ... -->
</dict>
</plist>
```

## 변경 영향 범위

| 파일 | 변경 내용 |
|---|---|
| `Cargo.toml` | `plist = "1"` 의존성 추가 |
| `src/cli.rs` | `Editor` enum에 `ITerm2` 추가, `Target` enum에 `ITerm2` 추가 |
| `src/reader.rs` | iTerm2 프리셋 스캔 함수 추가 |
| `src/converter_iterm2.rs` | 신규: iTerm2 plist → ThemeIR 변환 |
| `src/target/iterm2.rs` | 신규: ThemeIR → .itermcolors 변환 |
| `src/target/mod.rs` | iTerm2 타겟 등록 |
| `src/main.rs` | iTerm2 입력 플로우 분기 추가 |
| `tests/` | iTerm2 입출력 테스트 추가 |

## Resolved Questions

- **DynamicProfiles 포맷 호환성**: .itermcolors 파일만 생성하고 Import 방법 안내. DynamicProfiles 심링크는 하지 않음 (포맷이 다르므로).
- **theme_type 감지**: 입력 시 사용자에게 dark/light 선택을 물어봄 (인터랙티브 프롬프트 또는 `--theme-type dark|light` 플래그). .itermcolors에는 메타데이터가 없으므로 자동 감지 대신 명시적 선택.
- **추가 색상 키**: 출력 시 파생 매핑 — Bold Color = Foreground, Link Color = Accent, Selected Text Color = Foreground, Cursor Text Color = Background. 나머지(Badge, Underline, Tab, Cursor Guide)는 생략. 입력 시 추가 키는 무시.
- **macOS 전용**: 입력 소스 감지는 macOS에서만 활성화. 출력 타겟(.itermcolors 생성)은 모든 OS에서 가능 (다른 기기로 전송 가능).
- **Custom Color Presets 키 부재**: iTerm2가 설치되어 있어도 커스텀 프리셋이 없으면 `Custom Color Presets` 키 자체가 없을 수 있음. 키 없으면 "커스텀 프리셋 없음" 안내 후 스킵.
- **Color Space 호환**: `sRGB`와 `Calibrated` 모두 동일하게 float → hex 변환. 두 색공간 모두 0.0~1.0 범위의 RGB 값이므로 변환 로직 동일.
