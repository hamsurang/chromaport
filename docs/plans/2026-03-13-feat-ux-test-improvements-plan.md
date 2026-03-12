---
title: "feat: E2E UX 테스트 기반 종합 개선"
type: feat
status: active
date: 2026-03-13
deepened: 2026-03-13
origin: docs/brainstorms/2026-03-12-ux-test-improvements-brainstorm.md
---

# feat: E2E UX 테스트 기반 종합 개선

## Enhancement Summary

**Deepened on:** 2026-03-13
**Agents used:** Architecture Strategist, Performance Oracle, Security Sentinel, Pattern Recognition Specialist, Code Simplicity Reviewer, ratatui Research, ureq Research

### Key Improvements from Deepening
1. **ureq 3.x API 수정**: `Error::Status(code, _)` → `Error::StatusCode(u16)` (ureq 3.2.0 실제 API)
2. **HTTP 분기 단순화**: 5개 → 3개 (404, 기타 상태코드, Transport)
3. **`rgb_to_hsl` 제거**: 이미 `color.rs:26`에 존재 — 불필요한 중복 방지
4. **slug 충돌 방지 별도 이슈로 분리**: display-only 중복 구분만 이번 PR에 포함
5. **`ColorPickerMode` enum 도입**: `bool` + `String` 대신 명시적 상태 머신
6. **`ColorPicker`에서 `Copy` derive 제거**: `String` 필드 추가로 인해 필수
7. **help bar 공통 함수 추출 취소**: 인라인이 더 간결 (내용이 약간 다름)
8. **`interactive.rs`에 새 MultiSelect 함수**: `apply.rs`에 inquire 직접 호출 금지

### New Considerations Discovered
- `existing_theme_path`는 파일 존재만 확인 — 실제 앱 활성화 여부와 다를 수 있음
- Apply TUI 필터 구현 시 `ApplyApp` 상태 구조체 도입 권장 (기존 `PreviewApp` 패턴 준수)
- `created_at`은 스토리지 메타데이터 — `ThemeIR`에 추가하되 non-semantic으로 문서화

---

## Overview

E2E UX 테스트 리포트(`docs/reports/2026-03-12-e2e-ux-test-report.md`)에서 발견된 8개 이슈를 해결하는 종합 UX 개선. 인프라(프리셋 매니페스트 배포) → 에러 메시지 → UI/UX 디테일 순으로 단일 PR에서 진행.

## Problem Statement / Motivation

- Flow 4-5(presets list/install)가 완전히 불가 — `assets/presets/index.json`이 main 브랜치에 없어 404 반환
- 404 에러에 "Check your internet connection" 표시 — 사용자를 잘못된 방향으로 유도
- 테마 목록 중복, Preview 제목 빈 이름, 메타데이터 부족 등 UX 디테일 이슈
- color picker에 hex 직접 입력 불가 — 정밀 색상 지정 제한

## Proposed Solution

8개 이슈를 3단계로 나누어 순차 구현. 각 단계는 독립 커밋으로 분리.

### Phase 1: 인프라 복구 (커밋 1-2)

#### 1-1. `assets/presets/index.json` main 브랜치 배포

- 현재 `import-apply` 브랜치에만 존재하는 `assets/presets/index.json`을 PR 머지를 통해 main에 반영
- `assets/presets/` 디렉토리의 개별 테마 IR JSON도 함께 배포

> 파일: `assets/presets/index.json`, `assets/presets/*.json`

#### 1-2. HTTP 상태별 에러 메시지 분기

`src/presets.rs`의 `fetch_manifest()`와 `fetch_theme_ir()` 양쪽 모두에 적용.

### Research Insights

**ureq 3.2.0 실제 API** (ureq 리서치 결과):
- ureq 3.x는 `Error::StatusCode(u16)` variant를 사용 (2.x의 `Error::Status(code, response)`와 다름)
- Transport 에러는 `Error::HostNotFound`, `Error::Timeout`, `Error::ConnectionFailed`, `Error::Io` 등 개별 variant로 분리
- `#[non_exhaustive]` enum이므로 `_` 와일드카드 필수
- JSON 읽기: `response.body_mut().read_json::<T>()` (2.x의 `into_json()` 아님)
- 참고: `docs/solutions/build-errors/ureq-3x-api-migration.md`

**에러 분기 단순화** (Simplicity Review 반영):
- 429 분기 제거: `raw.githubusercontent.com`은 API 아닌 CDN — rate limit 사실상 없음
- 5xx 별도 분기 제거: "try again later"는 일반 상태코드 메시지로 충분
- 3개 분기로 축소: 404, 기타 상태코드, 네트워크 에러

```rust
// src/presets.rs — fetch_manifest() 에러 처리 (ureq 3.2.0 API)
fn fetch_manifest(agent: &ureq::Agent) -> Result<Manifest> {
    match agent.get(MANIFEST_URL).call() {
        Ok(mut resp) => Ok(resp.body_mut().read_json::<Manifest>()?),
        Err(ureq::Error::StatusCode(404)) => {
            bail!("Preset catalog not found. It may not be available yet.")
        }
        Err(ureq::Error::StatusCode(code)) => {
            bail!("Failed to fetch preset catalog (HTTP {code}).")
        }
        Err(e @ (ureq::Error::Timeout(_)
            | ureq::Error::HostNotFound
            | ureq::Error::ConnectionFailed
            | ureq::Error::Io(_))) => {
            bail!("Network error: {e}. Check your internet connection.")
        }
        Err(e) => {
            bail!("Failed to fetch preset catalog: {e}")
        }
    }
}
```

**`fetch_theme_ir()`에도 동일 패턴 적용** (SpecFlow 분석 반영):
- 테마 slug 404는 매니페스트 데이터 불일치 → "Theme '{slug}' not found in preset catalog."
- 네트워크 에러는 동일 메시지

> 파일: `src/presets.rs` (fetch_manifest L39-49, fetch_theme_ir L51-62)

---

### Phase 2: TUI/UX 개선 (커밋 3-7)

#### 2-1. 테마 목록 출처 표시로 중복 구분

**변경점 (display-only):**
1. `ThemeEntry`에 `extension_name: String` 필드 추가 (`src/reader.rs`)
   - `parse_extension_themes`에서 `ext_dir` 경로 또는 `pkg["name"]`으로 추출
   - 추출 실패 시 빈 문자열 → suffix 없이 표시
2. `PreviewApp::new()`에서 중복 이름 감지 + 레이블 사전 구성 (`src/preview/app.rs`)
   - `HashMap<String, Vec<usize>>`로 O(n) 한 번 순회하여 중복 감지
   - 유일한 이름 → `"One Dark Pro"`
   - 중복 이름 → `"Material Theme (equinox-theme)"`, `"Material Theme (community-material-theme)"`
   - **레이블 사전 구성은 `PreviewApp::new()`에서 수행** — 렌더러(`ui.rs`)는 순수 표시 함수 유지 (Architecture Review)

### Research Insights

**slug 충돌은 별도 이슈로 분리** (Simplicity Review + Architecture Review 합의):
- 동명 테마 저장 시 slug 충돌(silent overwrite)은 실제 버그이나, 이번 8개 이슈 범위 밖
- `save_ir` API 변경(`slug: &str` 파라미터 추가)까지 필요하므로 별도 PR로 분리
- 이번 PR에서는 display-only 중복 구분만 구현

**Performance 최적화** (Performance Oracle 반영):
- 레이블은 `PreviewApp::new()`에서 1회 캐시 → 키스트로크당 재생성 방지
- `filtered_indices`에 `Vec::clear()` + `extend` 패턴 사용 → capacity 재활용

> 파일: `src/reader.rs`, `src/preview/app.rs`

#### 2-2. Preview 제목 빈 이름 → "Untitled" 표시

`src/preview/ui.rs:127`에서 렌더링 시 빈 이름 체크:

```rust
let display_name = if ir.name.is_empty() { "Untitled" } else { &ir.name };
let title = format!(" Preview: {} ({type_label}) ", display_name);
```

- display-only 변경 — IR 데이터에는 영향 없음
- `derive_palette()`가 빈 이름을 설정하는 것은 그대로 유지 (이름은 나중에 inquire로 입력)
- **"Untitled" 표시는 `ui.rs`에만** — IR이나 `derive_palette()`에 추가하지 않음 (Architecture Review: 프레젠테이션 로직은 UI 레이어)

> 파일: `src/preview/ui.rs` (render_preview L120-133)

#### 2-3. 저장된 테마 메타데이터 표시 (Dark/Light + 생성일)

**ThemeIR 스키마 변경:**

```rust
// src/ir.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeIR {
    // ... 기존 필드 ...
    /// Storage metadata — not semantic IR data. Set by store::save_ir().
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,  // ISO 8601: "2026-03-13"
}
```

- `Option<String>`으로 backward-compatible (기존 IR 파일 역직렬화 실패 방지)
- `store::save_ir()`에서 `created_at`이 `None`이면 현재 날짜로 자동 설정
- `skip_serializing_if`로 기존 파일에 불필요한 필드 추가 방지

### Research Insights

**설계 결정** (Architecture Review):
- `created_at`은 스토리지 메타데이터이지 semantic IR이 아님. 이상적으로는 `StoredTheme { ir: ThemeIR, created_at }` 래퍼가 맞으나, 모든 함수 시그니처 변경이 필요하므로 pragmatic하게 `ThemeIR`에 추가
- `converter.rs`나 `color.rs`에서 `created_at`을 설정하지 않도록 주의 (store layer만 설정)
- 테스트: `created_at` 없는 기존 JSON → `None`으로 역직렬화 확인 필수

**날짜 형식**: ISO 8601 문자열 (`"2026-03-13"`). Relative date ("3일 전")는 렌더 시 `now()` 참조가 필요하므로 복잡도 증가 — 절대 날짜 사용.

**Apply TUI 표시:**

```
┌ Saved Themes ──────────────────────────┐
│ > Ayu Light          Light  2026-03-12 │
│   E2E Test Theme     Dark   2026-03-13 │
│   Catppuccin Mocha   Dark              │  ← 날짜 없는 기존 IR
└────────────────────────────────────────┘
```

> 파일: `src/ir.rs`, `src/store.rs`, `src/preview/apply_preview.rs`

#### 2-4. Apply 타겟 "already applied" 마커

**현재 동작:** `apply.rs:58-72`에서 `unapplied` 타겟만 필터링하여 MultiSelect에 전달.

**변경:**
- 모든 타겟을 MultiSelect에 표시하되, 적용 완료 타겟은 `(applied)` 마커 추가
- 적용 완료 타겟은 **기본 선택 해제** 상태로 표시 (사용자가 재적용 가능)
- 재적용 선택 시 기존 `confirm_overwrite` 로직 재사용

```
? Select targets to apply:
> [ ] Superset (applied)
  [x] Warp
  [ ] Ghostty (applied)
```

### Research Insights

**`interactive.rs`에 새 함수 추가** (Architecture Review):
- `inquire::MultiSelect` 호출은 `interactive.rs`에만 — `apply.rs`에 직접 호출 금지
- 새 함수: `select_targets_with_applied(available: &[Target], applied: &[usize]) -> Result<Vec<Target>>`
- `(applied)` 레이블 포맷팅도 이 함수 내부에서 처리

**엣지 케이스** (SpecFlow 분석):
- `existing_theme_path`는 파일 존재만 확인 — 앱에서 실제 활성화 여부와 다를 수 있음. 이 한계는 수용 (완벽한 감지는 앱별 설정 파싱 필요)
- 단일 미적용 타겟 auto-select 패스 유지 (모든 타겟 applied일 때만 early return)
- 모든 타겟이 applied이면 기존 "already applied to all" 메시지 유지

> 파일: `src/apply.rs`, `src/interactive.rs`

#### 2-5. Apply TUI 필터 기능 + Help bar 일관성

**문제:** Apply TUI에는 필터 기능이 없는데 "Type to filter" 힌트를 추가하면 혼란 유발.

**결정:** Apply TUI에도 필터 기능을 구현한 후 help bar에 힌트 추가. 힌트와 기능은 반드시 함께 출시.

**변경:**
1. `apply_preview.rs`에 필터 상태 추가 (인라인):
   - `filter: String`, `filtered_indices: Vec<usize>` 필드
   - `add_filter_char()`, `delete_filter_char()`, `clear_filter()` 메서드
   - 이벤트 루프에 문자 입력 → 필터 업데이트 로직
2. Help bar 업데이트 (인라인 유지):
   - 메인 TUI: `↑/↓ navigate  Enter select  q quit  Type to filter  Esc clear` (변경 없음)
   - Apply TUI: 동일하게 `Type to filter  Esc clear` 추가
   - Color picker: `←/→ adjust  ↑/↓ switch  Enter confirm  Esc back` (별도 — 다른 인터랙션 모드)

### Research Insights

**공통 help bar 함수 추출 취소** (Simplicity Review):
- 두 help bar는 내용이 약간 다르고, 각각 ~10줄에 불과
- 파라미터화된 공통 함수는 오히려 가독성 저하 (조건부 힌트 표시 로직)
- 인라인 유지가 더 간결하고 명확

**`ApplyApp` 구조체 도입 권장** (Architecture Review):
- 현재 `select_ir_with_preview`가 로컬 변수로 상태 관리 중
- 필터 상태 추가 시 `PreviewApp` 패턴과 동일한 상태 머신이 됨
- `ApplyApp` 구조체로 추출하면 `PreviewApp`과 일관된 패턴 유지
- 단, 이번 PR에서는 선택적 — 인라인으로 시작 후 복잡해지면 추출

> 파일: `src/preview/apply_preview.rs`

---

### Phase 3: Color Picker Hex 입력 (커밋 8)

#### 3-1. Hex 입력 모드

### Research Insights

**`rgb_to_hsl` 이미 존재** (Simplicity Review — critical fix):
- `color.rs:26`에 `pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64)` 존재
- 라운드트립 테스트도 통과 중 (`color.rs:316-331`)
- **새로 구현하지 않음** — 기존 함수 사용

**`ColorPickerMode` enum 도입** (Architecture Review):
- `bool` + `String` 대신 명시적 상태 머신으로 무효 상태 방지
- hex 입력 버퍼는 `HexInput` variant에만 존재

**`Copy` derive 제거 필수** (Pattern Recognition):
- 현재 `ColorPicker`에 `Copy` derive 존재
- `String` 필드(`HexInput { buffer }`) 추가 시 `Copy` 불가
- `ColorPicker` 사용처에서 Copy 의미론 사용 여부 확인 후 제거

**UI 설계:**
- `#` 키로 hex 입력 모드 진입 (help bar에 표시)
- 입력 중 화면: 슬라이더 아래에 `# ______` 텍스트 필드 표시
- 커서 렌더링: `frame.set_cursor_position()` 사용
- 6자리 입력 완료 + Enter → 슬라이더 HSL 값 업데이트
- `Esc`로 hex 모드 취소, 슬라이더 모드 복귀
- `Backspace`로 문자 삭제

**ColorPicker 상태 확장:**

```rust
// src/preview/color_picker.rs

#[derive(Debug, Clone)]  // Copy 제거됨
pub enum ColorPickerMode {
    Sliders,
    HexInput { buffer: String },  // # 제외, 최대 6자
}

pub struct ColorPicker {
    pub h: f64,
    pub s: f64,
    pub l: f64,
    pub active: usize,
    pub mode: ColorPickerMode,
}
```

**입력 검증:**
- 유효한 hex 문자만 허용 (`c.is_ascii_hexdigit()` && `buffer.len() < 6`)
- 6자리 미만은 미리보기 업데이트 안 함 (슬라이더 위치 유지)
- 6자리 도달 + Enter: `HexColor::parse()` → `to_rgb()` → `rgb_to_hsl()` → 슬라이더 업데이트
- **반드시 `HexColor::parse()`를 검증자로 사용** (Architecture Review: 중앙화된 검증자 패턴)
- 3자리 shorthand 미지원 — 항상 6자리 (단순성 우선)

**Hex → HSL 변환 경로 (기존 API 사용):**
```rust
// 기존 함수만 사용 — 새 함수 불필요
let hex = HexColor::parse(&format!("#{}", buffer))?;  // ir.rs
let (r, g, b) = hex.to_rgb();                          // ir.rs
let (h, s, l) = color::rgb_to_hsl(r, g, b);           // color.rs:26 (이미 존재)
picker.h = h;
picker.s = s;
picker.l = l;
```

**Help bar 업데이트:**
- 슬라이더 모드: `←/→ adjust  ↑/↓ switch  Enter confirm  Esc back  # hex`
- Hex 모드: `Type hex (6 digits)  Enter confirm  Esc cancel  Backspace delete`

> 파일: `src/preview/color_picker.rs`, `src/preview/create.rs`

---

## Technical Considerations

### Backward Compatibility
- `ThemeIR.created_at: Option<String>` + `#[serde(default)]` — 기존 IR 파일 역직렬화 안전
- 테마 display 이름 변경(suffix 추가)은 UI에만 영향 — 저장된 IR 파일, slug에는 영향 없음

### Performance (Performance Oracle 반영)
- 테마 중복 감지: `HashMap<String, Vec<usize>>`로 O(n), `PreviewApp::new()`에서 1회 실행
- 레이블 캐시: `Vec<String>` 1회 할당, 키스트로크당 재생성 방지
- 필터 업데이트: `filtered_indices.clear()` + `extend` 패턴으로 capacity 재활용
- hex `rgb_to_hsl` 변환: 단일 연산 — 성능 영향 없음

### Security (Security Sentinel 반영)
- `extension_name` 추출: 경로 기반이므로 path traversal 위험 없음 (표시 용도만)
- hex 입력: `HexColor::parse()`가 유일한 검증자 — bespoke 검증 금지
- `created_at` 형식: `Option<String>`이므로 파싱 실패 시 panic 없음. 향후 날짜 파싱 코드 추가 시 `NaiveDate::parse_from_str` + fallback 필요

### Known Issue: Slug Collision (별도 PR)
- 동명 테마 저장 시 같은 slug → silent overwrite 위험 (Security: data loss vector)
- 해결 방안: `save_ir(ir, slug)` API 변경 + 충돌 감지
- 이번 PR 범위 밖 — 별도 이슈로 추적

---

### 테스트 전략
- ureq 에러 메시지 분기: 각 `Error` variant에 대한 단위 테스트
- 테마 중복 감지 로직: 0개, 1개, 복수 중복 케이스
- `ThemeIR` 역직렬화: `created_at` 없는 JSON → `None` 파싱 확인
- `HexColor` 파싱: 기존 테스트 + hex 모드 6자리 입력 시나리오
- `ColorPickerMode` 전환: Sliders ↔ HexInput 전환 + Esc 취소
- `rgb_to_hsl` 라운드트립: 기존 테스트 이미 통과 (`color.rs:316-331`)

---

## Acceptance Criteria

### Phase 1: 인프라 복구
- [ ] `assets/presets/index.json` 및 개별 프리셋 IR이 main 브랜치에 존재
- [ ] `chromaport presets list` 실행 시 프리셋 목록 정상 출력
- [ ] `chromaport presets install` 실행 시 프리셋 다운로드 및 저장 정상 동작
- [ ] 네트워크 에러 시 "Check your internet connection" 메시지 표시
- [ ] 404 시 "catalog not found" 메시지 표시
- [ ] 기타 HTTP 에러 시 상태 코드 포함 메시지 표시

### Phase 2: TUI/UX 개선
- [ ] 동명 테마에 확장 출처 suffix 표시 (예: `Material Theme (equinox-theme)`)
- [ ] 유일한 이름은 suffix 없이 표시
- [ ] create 플로우 Preview 제목: `Preview: Untitled (Dark)` (이름 입력 전)
- [ ] apply TUI에 Dark/Light 타입 표시
- [ ] `created_at`이 있는 IR은 날짜 표시, 없는 IR은 빈칸
- [ ] apply 타겟 MultiSelect에 `(applied)` 마커 표시
- [ ] applied 타겟 재선택 시 overwrite 확인 프롬프트
- [ ] apply TUI에 필터 기능 동작 (Type to filter + Esc clear)
- [ ] apply TUI help bar에 필터 힌트 포함

### Phase 3: Color Picker
- [ ] `#` 키로 hex 입력 모드 진입
- [ ] 6자리 hex 입력 + Enter 시 슬라이더 HSL 값 자동 업데이트
- [ ] `Esc`로 hex 모드 취소 시 슬라이더 모드 복귀
- [ ] 유효하지 않은 문자 입력 무시
- [ ] hex 입력 검증에 `HexColor::parse()` 사용

---

## Dependencies & Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| `ThemeIR` 스키마 변경으로 기존 IR 역직렬화 실패 | High | `Option` + `#[serde(default)]` 사용 |
| apply TUI 필터 구현이 예상보다 복잡 | Medium | 인라인으로 시작, 복잡해지면 `ApplyApp` 추출 |
| `ColorPicker`에서 `Copy` 제거 시 기존 코드 영향 | Low | 사용처 확인 — Clone으로 대체 |
| hex 입력 모드의 ratatui 텍스트 입력 | Low | 6자 고정이므로 간단한 인라인 버퍼로 충분 |
| `index.json` 배포가 main 브랜치 머지에 의존 | Low | PR 머지 시 자동 반영 |

---

## Version Bump

이 PR은 새 기능(feat)을 포함하므로 minor 버전 bump 필요.
- 현재: `0.6.0`
- PR 머지 후: `0.7.0`
- Git tag: `v0.7.0`

---

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-03-12-ux-test-improvements-brainstorm.md](docs/brainstorms/2026-03-12-ux-test-improvements-brainstorm.md) — 개선 범위, 중복 처리 방식(출처 표시), hex 입력 포함, 인프라 우선 접근 결정
- **E2E 테스트 리포트:** [docs/reports/2026-03-12-e2e-ux-test-report.md](docs/reports/2026-03-12-e2e-ux-test-report.md)
- **Learnings 적용:**
  - `docs/solutions/build-errors/ureq-3x-api-migration.md` — ureq 3.x API: `body_mut().read_json()`, `Error::StatusCode`, feature flag
  - `docs/solutions/code-quality/code-review-central-theme-store-ux-refactoring.md` — DRY, 파라미터화, atomic ops, enum 설계, `anyhow::bail!`
- **관련 파일:**
  - `src/presets.rs` — 프리셋 fetch/에러 처리
  - `src/preview/ui.rs:127` — Preview 제목 렌더링
  - `src/preview/app.rs:75-84` — 테마 목록/saved flags
  - `src/preview/apply_preview.rs:131-144` — apply TUI help bar
  - `src/preview/color_picker.rs:11-50` — HSL 피커
  - `src/apply.rs:58-72` — 타겟 필터링
  - `src/ir.rs:218-237` — ThemeIR 구조체
  - `src/store.rs:78-111` — theme_slug 생성
  - `src/reader.rs` — ThemeEntry 구조체
  - `src/color.rs:26` — rgb_to_hsl (이미 존재)
  - `src/interactive.rs` — 프롬프트 추상화 (새 함수 추가 대상)
