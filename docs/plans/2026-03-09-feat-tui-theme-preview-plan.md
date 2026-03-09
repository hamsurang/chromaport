---
title: "feat: Add TUI theme preview with live color rendering"
type: feat
status: active
date: 2026-03-09
deepened: 2026-03-09
origin: docs/brainstorms/2026-03-09-theme-preview-brainstorm.md
---

# feat: Add TUI theme preview with live color rendering

## Enhancement Summary

**Deepened on:** 2026-03-09
**Research agents used:** architecture-strategist, performance-oracle, security-sentinel, code-simplicity-reviewer, pattern-recognition-specialist, best-practices-researcher, Context7 (ratatui, crossterm)

### Key Improvements (from deepening)

1. **Phase 0 추가**: crossterm 버전 호환성 검증 스파이크 (가장 큰 기술 리스크 조기 해소)
2. **모듈 3개로 축소**: `color.rs`와 `syntax.rs`를 `ui.rs`에 통합 (YAGNI)
3. **디바운스 제거**: 동기 변환 + 캐시로 충분 (변환 5-20ms, 캐시 후 즉시)
4. **256색 폴백 제거**: truecolor 미지원 시 inquire Select로 폴백 (2줄 env 체크)
5. **TEA 아키텍처 패턴**: ratatui 공식 권장 Model/Update/View 패턴 적용
6. **RAII TerminalGuard**: panic hook만이 아닌 Drop guard로 터미널 복원 보장
7. **보안 발견**: `include` 경로 검증 취약점 사전 수정 필요 (reader.rs)
8. **변환 클로저 패턴**: `&ThemeReader` 대신 `Fn(&ThemeEntry) -> Result<ThemeIR>` 로 테스트 용이성 확보

### Simplifications Applied

| 제거된 항목 | 이유 | 절감 LOC |
|-------------|------|----------|
| `src/preview/color.rs` | `ui.rs`에 3줄 헬퍼로 통합 | ~50 |
| `src/preview/syntax.rs` | `ui.rs`에 정적 데이터로 통합 | ~30 |
| 256색 폴백 알고리즘 | YAGNI - inquire로 폴백 | ~60 |
| 100ms 디바운스 | YAGNI - 동기 변환+캐시로 충분 | ~30 |
| `--no-preview` 플래그 | YAGNI - `--yes`와 env 감지로 충분 | ~20 |
| Phase 5 (독립 phase) | Phase 2에 통합 | 계획 오버헤드 |

---

## Overview

테마 import 파이프라인의 테마 선택 단계를 ratatui+crossterm 기반 TUI로 교체하여, 커서 이동 시 해당 테마의 색상을 실시간으로 미리 볼 수 있는 기능을 추가한다.

기존 `inquire::MultiSelect` → `ratatui` 기반 좌/우 분할 single-select TUI로 변경. 왼쪽 패널에 테마 리스트(type-to-filter 지원), 오른쪽 패널에 target별 preview (팔레트, UI 색상, TypeScript 코드 스니펫).

**Breaking change**: multi-select가 single-select로 변경된다. 여러 테마를 import하려면 여러 번 실행해야 한다. 이는 preview UX를 위한 의도적 결정이다 (see brainstorm).

## Problem Statement

현재 chromaport는 테마를 이름 목록에서 선택하는데, 실제 색상을 확인하려면 import 후 직접 대상 앱에서 확인해야 한다. 수십~수백 개 테마 중 원하는 것을 찾으려면 여러 번 시행착오를 거쳐야 하며, 이는 사용자 경험을 크게 저하시킨다.

## Proposed Solution

```
┌─ Themes (type to filter) ─┐┌─ Preview: One Dark Pro (Dark) ────────────┐
│ > One Dark Pro             ││ bg:#282c34  fg:#abb2bf  accent:#528bff   │
│   Dracula                  ││                                          │
│   Monokai Pro              ││ Normal: ██ ██ ██ ██ ██ ██ ██ ██         │
│   GitHub Dark              ││ Bright: ██ ██ ██ ██ ██ ██ ██ ██         │
│   Catppuccin Mocha         ││                                          │
│   Solarized Dark           ││ const greet = (name: string): void => {  │
│                            ││   const msg = `Hello, ${name}!`;         │
│                            ││   console.log(msg);                      │
│                            ││ };                                       │
│                            ││ // Call the function                      │
│                            ││ greet("world"); // 42                    │
└────────────────────────────┘└──────────────────────────────────────────┘
 ↑/↓/j/k navigate  Enter select  Esc quit  Type to filter
```

## Technical Approach

### Architecture

TEA (The Elm Architecture) 패턴으로 구현: Model(상태) / Update(상태 전이) / View(렌더링) 분리. ratatui 공식 권장 패턴이며, 단일 화면의 preview 기능에 적합하다.

**모듈 구조 (3개 파일):**

```
src/
├── preview/
│   ├── mod.rs    # 공개 API, 터미널 setup/teardown, TerminalGuard, 이벤트 루프
│   ├── app.rs    # Model: 앱 상태, Update: 입력 처리/필터링/캐싱
│   └── ui.rs     # View: 레이아웃, 팔레트 위젯, 코드 스니펫, 색상 변환 헬퍼
```

> `target/` 디렉토리 패턴을 따라 `mod.rs`가 공개 API, 내부 파일은 private.
> `color.rs`(3줄 헬퍼)와 `syntax.rs`(정적 데이터)는 `ui.rs`에 통합하여 불필요한 모듈 경계 제거.

**데이터 흐름 (TEA):**

```
                    ┌──────────────────────┐
                    │   Model (app.rs)     │
                    │  themes, selected,   │
                    │  filter, cache       │
                    └───┬──────────────┬───┘
                        │              │
              Update    │              │  View
          (key event)   │              │  (render)
                        ▼              ▼
                 ┌────────────┐  ┌──────────┐
                 │  app.rs    │  │  ui.rs   │
                 │ handle_key │  │ draw()   │
                 └────────────┘  └──────────┘
```

**변환 클로저 패턴** (테스트 용이성 확보):

```rust
struct PreviewApp<F: Fn(&ThemeEntry) -> Result<ThemeIR>> {
    convert: F,
    themes: Vec<ThemeEntry>,
    // ...
}
```

`&ThemeReader` 대신 클로저를 받아 reader/converter로부터 디커플링. 테스트 시 mock 클로저를 제공하면 된다.

### Implementation Phases

#### Phase 0: crossterm 호환성 검증 스파이크

> **이 Phase가 모든 후속 작업을 블로킹한다.** inquire 0.7은 crossterm 0.27을, ratatui 0.29+는 crossterm 0.28을 사용한다. 버전 충돌 시 전체 접근 방식을 재검토해야 한다.

**Tasks:**

- [ ] 최소 Cargo 프로젝트 생성: `inquire 0.7` + `ratatui` 동시 의존
- [ ] `cargo tree -i crossterm` 으로 crossterm 버전 중복 확인
- [ ] 동작 검증: inquire `Select` 프롬프트 실행 → ratatui alternate screen 진입 → 정상 종료
- [ ] 터미널 상태 깨짐 여부 확인 (raw mode, cursor visibility)

**판단 기준:**

| 결과 | 대응 |
|------|------|
| 단일 crossterm 버전 | 그대로 진행 |
| 이중 crossterm, 순차 사용 시 정상 | defensive `disable_raw_mode()` 추가 후 진행 |
| 이중 crossterm, 상태 오염 | inquire를 ratatui 위젯으로 대체 검토 (Option A) |

**Success criteria:** inquire → ratatui → stdout 전환이 터미널 상태를 깨뜨리지 않음

### Research Insights (Phase 0)

- ratatui는 `ratatui::crossterm::*` 로 re-export 제공. crossterm을 Cargo.toml에서 직접 의존하지 말고 ratatui의 re-export를 사용하면 버전 단일화에 유리
- inquire와 ratatui는 반드시 **순차적으로** 사용 (concurrent 사용 금지). inquire 완료 후 ratatui 시작
- ratatui 진입 전 방어적 `crossterm::terminal::disable_raw_mode()` 호출 (idempotent, 안전)

---

#### Phase 1: Foundation + 렌더링 (Phases 1+2 통합)

`HexColor::to_rgb()` 추가, 의존성 추가, preview 렌더링 위젯 전체 구현.

**Tasks:**

- [ ] `HexColor::to_rgb(&self) -> (u8, u8, u8)` 메서드 추가 (`src/ir.rs`)
  - `#RGB` → 각 자리 더블링 (`#F0A` → `#FF00AA`)
  - `#RRGGBB` → 직접 파싱
  - `#RRGGBBAA` → 알파 제거 (기존 ghostty.rs의 `&s[..7]` 패턴과 동일)
- [ ] `Cargo.toml`에 `ratatui` 추가 (crossterm은 ratatui re-export 사용)
- [ ] 색상 변환 헬퍼 (`src/preview/ui.rs` 상단)
  ```rust
  fn to_color(hex: &HexColor) -> ratatui::style::Color {
      let (r, g, b) = hex.to_rgb();
      Color::Rgb(r, g, b)
  }
  ```
- [ ] TypeScript 코드 스니펫 + `TokenKind` enum (`src/preview/ui.rs`)
  - `TokenKind` enum: `Keyword`, `String`, `Function`, `Number`, `Comment`, `Type`, `Punctuation`, `Plain`
  - 하드코딩된 8-12줄 TypeScript 코드를 `Vec<(&str, TokenKind)>` 으로 정의
  - `fn resolve_color(kind: TokenKind, ir: &ThemeIR) -> Color` 로 매핑:
    - `Keyword` → `ir.terminal.normal.blue`
    - `String` → `ir.terminal.normal.green`
    - `Function` → `ir.terminal.normal.yellow`
    - `Number` → `ir.terminal.normal.magenta`
    - `Comment` → `ir.muted_fg`
    - `Type` → `ir.terminal.normal.cyan`
    - `Punctuation` / `Plain` → `ir.terminal.foreground`
  - `syntax.rs` 분리 불필요: 정적 데이터 + 1개 함수는 `ui.rs`에 충분
- [ ] ANSI 16색 팔레트 위젯 (`src/preview/ui.rs`)
  - Normal 8색 + Bright 8색을 `█` 블록으로 렌더링
  - 2행 x 8열 레이아웃
  - `StatefulWidget` 불필요, 단순 `Widget` 구현
- [ ] Target별 preview 구성 (`src/preview/ui.rs`)
  - 공통: bg, fg, accent + ANSI 16색 팔레트 + 코드 스니펫
  - Superset 전용: `if target == Superset { render_superset_extras() }` 단일 조건문
    - sidebar_bg, sidebar_fg, border, chart_colors(5색 스와치) 추가 표시
  - trait/전략 패턴 불필요 — 단순 `if` 블록으로 충분
- [ ] Truecolor 감지 (`src/preview/ui.rs` 또는 `mod.rs`)
  ```rust
  fn supports_truecolor() -> bool {
      matches!(
          std::env::var("COLORTERM").as_deref(),
          Ok("truecolor") | Ok("24bit")
      )
  }
  ```
  - truecolor 미지원 시 TUI 생략, inquire `Select`로 폴백
  - `NO_COLOR` 환경변수도 존중 (https://no-color.org/)
- [ ] 좌/우 분할 레이아웃
  ```rust
  let layout = Layout::horizontal([
      Constraint::Percentage(35),  // 테마 리스트
      Constraint::Percentage(65),  // preview pane
  ]).split(frame.area());
  ```
- [ ] Phase 1 테스트
  - `HexColor::to_rgb()` 단위 테스트 (3/6/8자리, 대소문자)
  - ratatui `TestBackend` + `insta` 스냅샷 테스트 (이미 dev-dep에 있음)
  - `TokenKind` → color 매핑 테스트
  - `test_fixtures::make_test_ir()` 재활용

**핵심 파일:** `src/ir.rs`, `src/preview/ui.rs`, `Cargo.toml`

**Success criteria:** `TestBackend`에서 올바른 레이아웃 + 팔레트 + 코드 스니펫 렌더링

### Research Insights (Phase 1)

- ratatui `List` 위젯 + `ListState`로 선택 상태 관리. `highlight_style`과 `highlight_symbol(">")`로 커서 표시
- `StatefulWidget::render(list, area, buf, &mut state)` 패턴으로 상태 기반 렌더링
- `Block::bordered().title()` 로 패널 테두리
- `Paragraph::new(vec![Line::from(vec![Span::styled(...)])])` 로 멀티 색상 텍스트
- ratatui는 double-buffering + diff를 내부적으로 수행하므로, 매 프레임 전체를 다시 렌더해도 성능 영향 없음 (immediate mode)

---

#### Phase 2: TUI 앱 + Lazy 변환

인터랙티브 TUI 앱 로직, 키보드 입력 처리, lazy theme 변환 + 캐싱, 터미널 안전성.

**Tasks:**

- [ ] RAII `TerminalGuard` (`src/preview/mod.rs`)
  ```rust
  struct TerminalGuard;

  impl TerminalGuard {
      fn new() -> Result<Self> {
          crossterm::terminal::enable_raw_mode()?;
          crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;
          Ok(Self)
      }
  }

  impl Drop for TerminalGuard {
      fn drop(&mut self) {
          let _ = crossterm::execute!(std::io::stderr(), LeaveAlternateScreen);
          let _ = crossterm::terminal::disable_raw_mode();
      }
  }
  ```
  - Drop guard로 정상 종료, `?` 에러 전파, panic 모두에서 터미널 복원 보장
  - 별도로 panic hook도 설치 (abort 시나리오 대비, 기존 hook 체이닝)
- [ ] SIGTSTP (Ctrl+Z) 차단: TUI 세션 동안 무시 (짧은 세션이므로 충분)
- [ ] TUI 앱 상태 구조체 (`src/preview/app.rs`)
  ```rust
  struct PreviewApp<F: Fn(&ThemeEntry) -> Result<ThemeIR>> {
      convert: F,
      themes: Vec<ThemeEntry>,
      filtered: Vec<usize>,
      selected: usize,
      filter_text: String,
      cache: HashMap<PathBuf, ThemeIR>,
      target: Target,
      active_id: Option<String>,
      error_msg: Option<String>,  // 변환 실패 시 에러 표시
  }
  ```
- [ ] 키보드 입력 처리 (`src/preview/app.rs`)
  - `↑`/`↓`/`j`/`k`: 탐색
  - `Enter`: 선택 확정
  - `Esc`/`q`: 취소 → `process::exit(0)` (inquire 패턴과 일치)
  - `Ctrl+C`: 취소 → `process::exit(130)` (inquire 패턴과 일치)
  - 문자 입력: `filter_text`에 추가, 목록 필터링 (`.to_lowercase().contains()` — 500개에도 마이크로초)
  - `Backspace`: `filter_text` 마지막 문자 삭제
  - `KeyEventKind::Press` 필터링 필수 (중복 이벤트 방지)
- [ ] 동기 Lazy 변환 + 캐싱 (디바운스 없음)
  - 커서 이동 시 즉시 변환 (5-20ms, 캐시 후 즉시)
  - `(self.convert)(&entry)` 호출 → 결과를 `cache`에 저장
  - 변환 실패 시 `error_msg`에 에러 표시, 테마 선택은 가능
  - 캐시는 `ThemeIR`만 저장 (raw JSON은 즉시 드롭)
- [ ] 이벤트 루프 (event-driven rendering)
  ```rust
  loop {
      terminal.draw(|f| ui::draw(f, &mut app))?;
      if crossterm::event::poll(Duration::from_millis(100))? {
          if let Event::Key(key) = event::read()? {
              if key.kind == KeyEventKind::Press {
                  match app.handle_key(key) {
                      AppResult::Selected(entry) => return Ok(entry),
                      AppResult::Quit => process::exit(0),
                      AppResult::Continue => {}
                  }
              }
          }
      }
  }
  ```
  - 이벤트 없으면 CPU 사용 0, 입력 시 즉시 반응
  - `Event::Resize` 자동 처리 (ratatui가 `terminal.draw()` 시 자동 재계산)
- [ ] Active 테마 표시: 목록 최상단에 정렬, `[active]` 마커 (기존 `select_themes()` 정렬 로직 재활용)
- [ ] Active 테마 캐시 pre-warm: TUI 진입 직후 active 테마를 미리 변환하여 첫 preview 즉시 표시
- [ ] Phase 2 테스트
  - 앱 상태 전이 테스트 (커서 이동, 필터링, 선택, 취소)
  - mock 변환 클로저로 캐시 동작 검증
  - `#[cfg(test)] mod tests` 블록 (per-file 컨벤션 준수)

**핵심 파일:** `src/preview/mod.rs`, `src/preview/app.rs`

**Success criteria:** 단독 TUI 앱이 실행, 키 입력 정상 반응, 터미널 100% 복원

### Research Insights (Phase 2)

- **디바운스 불필요 근거**: 변환은 5-20ms (30KB JSON 기준). ratatui의 프레임 예산 33ms(30fps)보다 작음. 캐시 적중 시 HashMap lookup은 나노초. 디바운스를 쓰면 오히려 100ms 동안 이전 preview가 보여서 UX가 나빠짐
- **이벤트 루프**: `poll(Duration::from_millis(100))` + `event::read()` 패턴이 ratatui 표준. 고정 framerate ticker 불필요
- **메모리**: ThemeIR ~1.2KB/개. 500개 캐시해도 ~600KB. 캐시 eviction 불필요 (세션 짧음)
- **성능 개선 기회 (P1)**: `converter::convert()`에서 `theme_json.clone()` 제거. 시그니처를 `convert(entry, theme_json: Value)` 로 변경하여 ownership 이전. 200-500KB 할당 절감

---

#### Phase 3: 파이프라인 통합

기존 main.rs 파이프라인에 TUI preview를 통합하고, 기존 multi-select를 교체한다.

**Tasks:**

- [ ] 파이프라인 순서 변경 (`src/main.rs`)
  - 기존: editor → themes → target → convert → write
  - 변경: editor → target → themes(with preview) → convert → write
  - `--yes` 경로도 동일하게 순서 변경 (기능적 차이 없으나 코드 일관성)
- [ ] `preview::select_theme_with_preview()` 공개 API (`src/preview/mod.rs`)
  ```rust
  pub fn select_theme_with_preview(
      themes: &[ThemeEntry],
      active_id: Option<&str>,
      target: &Target,
      convert: impl Fn(&ThemeEntry) -> Result<ThemeIR>,
  ) -> Result<ThemeEntry>
  ```
  > `interactive.rs`가 아닌 `preview/mod.rs`에 배치. `interactive.rs`는 100% inquire 전용이므로 ratatui 로직을 혼합하지 않음.
- [ ] `main.rs` 파이프라인에서 호출 교체
  - TTY + truecolor + 테마 2개 이상: `preview::select_theme_with_preview()` 사용
  - TTY + truecolor 미지원: inquire `Select` (single-select) 사용
  - TTY + 테마 1개: TUI 생략, 바로 해당 테마 선택
  - Non-TTY: 기존 동작 유지 (에러 또는 `--yes`로 자동 선택)
  - `--yes` 플래그: 기존대로 active 테마 자동 선택
- [ ] 반환 타입 변경: `Vec<ThemeEntry>` → `ThemeEntry` (single-select)
  - step 4-7의 반복 로직을 단일 처리로 변경
- [ ] inquire → ratatui 전환 안전성
  - inquire 프롬프트 완료 후 방어적 `disable_raw_mode()` 호출 (idempotent)
  - ratatui는 `TerminalGuard::new()`로 clean 진입
- [ ] Phase 3 통합 테스트 (`tests/cli.rs`)
  - `--yes` 모드에서 preview 미사용 확인
  - 파이프라인 순서 변경 후 기존 기능 정상 동작 확인

**핵심 파일:** `src/main.rs`, `src/preview/mod.rs`, `tests/cli.rs`

**Success criteria:** `chromaport` 실행 시 TUI preview 정상 표시, `--yes` 정상 동작

### Research Insights (Phase 3)

- inquire와 ratatui는 반드시 순차 사용 (concurrent 금지). inquire 완료 → ratatui 시작 순서
- 취소 시맨틱: 기존 inquire의 `handle_inquire_error()`는 `OperationCanceled` → `process::exit(0)`, `OperationInterrupted` → `process::exit(130)`. TUI에서도 동일하게 `Esc/q` → exit(0), `Ctrl+C` → exit(130) 적용
- `--yes` 경로에서도 파이프라인 순서를 일치시켜야 코드 가독성 유지

---

#### Pre-existing: `include` 경로 검증 보안 수정

> 보안 리뷰에서 발견: TUI preview 구현과 독립적이지만, TUI에서 lazy 변환 시 더 자주 호출되므로 사전 수정 권장.

- [ ] `reader.rs`의 `read_theme_json_with_includes()`에서 `include` 경로 검증 추가
  ```rust
  let include_abs = parent.join(include_path);
  if let Ok(canonical) = include_abs.canonicalize() {
      if !canonical.starts_with(&extension_dir_canonical) {
          // include path가 extension 디렉토리를 벗어남 — 무시
          continue;
      }
  }
  ```
  - 기존 `store::resolve_theme_path()`와 동일한 containment check 적용
  - null byte check도 추가: `if include_path.contains('\0') { continue; }`

## Alternative Approaches Considered

(see brainstorm: `docs/brainstorms/2026-03-09-theme-preview-brainstorm.md`)

| 접근 방식 | 장점 | 단점 | 결론 |
|-----------|------|------|------|
| **ratatui + crossterm** | 풍부한 UX, 레이아웃 관리, Rust 생태계 표준 | 의존성 추가 | **채택** |
| crossterm만 사용 | 의존성 최소화 | 레이아웃/스크롤 직접 구현 | 기각 |
| 순차적 출력 + inquire | 가장 간단 | 실시간 preview 불가 | 기각 |

## System-Wide Impact

### Interaction Graph

1. `main.rs` 파이프라인 순서 변경: target 선택이 theme 선택 전으로 이동
2. `preview::select_theme_with_preview()` 신규 공개 API (기존 `interactive::select_themes()` 대체)
3. step 4-7의 반복문 제거 (단일 테마 처리)
4. `reader.read_theme_json()` + `converter::convert()`가 TUI 내부에서 호출됨 (기존: 선택 후 호출)

### Error Propagation

- `reader.read_theme_json()` 실패 → preview pane에 에러 메시지, 테마 선택은 가능
- `converter::convert()` 실패 → 동일 처리 (실제로는 폴백 값 사용하므로 거의 발생 안 함)
- TUI panic → `TerminalGuard::drop()` + panic hook으로 터미널 복원
- TUI `Err` 반환 → `TerminalGuard::drop()`으로 터미널 복원 (panic hook 불필요)
- inquire → ratatui 전환 실패 → `anyhow::Result`로 상위 전파

### State Lifecycle Risks

- TUI 비정상 종료 → `TerminalGuard` Drop으로 해결 (panic hook + RAII 이중 보호)
- 변환 캐시는 session-only, `PreviewApp` 드롭 시 자동 해제
- 기존 `--yes` 경로는 TUI를 전혀 거치지 않으므로 영향 없음

### API Surface Parity

- `interactive::select_themes()` 제거, `preview::select_theme_with_preview()` 추가
- `interactive.rs`의 다른 함수(select_editor, select_target 등)는 변경 없음
- `Target` enum은 변경 없음 (read-only로 preview 렌더러에 전달)

### Integration Test Scenarios

1. `chromaport --yes --editor vscode --target ghostty` → TUI 미사용, 기존대로 active 테마 자동 선택
2. `echo | chromaport` → Non-TTY 감지, 적절한 에러 메시지 출력
3. 테마 0개인 에디터 → TUI 진입 전 bail ("No themes found")
4. 테마 JSON 파일 손상 → preview pane에 에러, 다른 테마는 정상

## Acceptance Criteria

### Functional Requirements

- [ ] TUI에서 좌/우 분할 레이아웃으로 테마 리스트와 preview가 표시된다
- [ ] 커서 이동 시 해당 테마의 preview가 즉시 갱신된다
- [ ] Preview에 테마 메타 정보(이름, 타입, bg/fg/accent), ANSI 16색 팔레트, TypeScript 코드 스니펫이 표시된다
- [ ] Target에 따라 preview 내용이 다르다 (Superset: UI/chart 색상 추가)
- [ ] Type-to-filter로 테마 목록을 실시간 필터링할 수 있다
- [ ] Enter로 테마 선택, Esc/q로 취소할 수 있다
- [ ] 선택된 테마가 정상적으로 write/activate 된다
- [ ] Active 테마가 목록 최상단에 `[active]` 마커와 함께 표시된다

### Non-Functional Requirements

- [ ] `--yes` 모드에서 TUI가 실행되지 않는다
- [ ] Non-TTY 환경에서 기존 동작으로 폴백한다
- [ ] Truecolor 미지원 시 inquire `Select`로 폴백 (preview 없이)
- [ ] TUI 종료 시 터미널 상태가 완전히 복원된다 (panic, Err, 정상 모두)
- [ ] SIGTSTP (Ctrl+Z) TUI 세션 중 차단

### Quality Gates

- [ ] `cargo test` 전체 통과
- [ ] `cargo fmt --check` 통과
- [ ] `cargo clippy --all-targets` 경고 없음
- [ ] `HexColor::to_rgb()` 단위 테스트 (3/6/8자리 hex)
- [ ] ratatui `TestBackend` + `insta` 스냅샷 테스트
- [ ] 앱 상태 전이 테스트 (탐색, 필터, 선택, 취소)
- [ ] 기존 CLI 통합 테스트 유지 (`tests/cli.rs`)

## Dependencies & Prerequisites

### 신규 의존성

| Crate | 용도 | 비고 |
|-------|------|------|
| `ratatui` | TUI 프레임워크 | crossterm을 re-export로 사용, 별도 crossterm 의존 불필요 |

### 호환성 확인 (Phase 0에서 검증)

- `inquire 0.7` (crossterm 0.27) + `ratatui` (crossterm 0.28) 공존 가능 여부
- 이중 crossterm 시 순차 사용으로 상태 오염 방지 가능 여부

## Risk Analysis & Mitigation

| 리스크 | 영향 | 확률 | 완화 |
|--------|------|------|------|
| crossterm 버전 충돌 | 터미널 깨짐 | **높** | **Phase 0 스파이크로 조기 검증**. 실패 시 inquire 대체 검토 |
| inquire → ratatui 전환 시 터미널 상태 | 터미널 깨짐 | 중 | TerminalGuard (RAII) + panic hook + 방어적 disable_raw_mode |
| `include` 경로 검증 취약 (기존) | 임의 파일 읽기 | 중 | 사전 수정: extension directory containment check 추가 |
| 대규모 테마(500+) 성능 | UX 열화 | 낮 | 동기 변환 + HashMap 캐시, active 테마 pre-warm |
| multi-select 제거 | 워크플로우 변경 | 낮 | 의도적 결정. 여러 번 실행으로 대체 |

## Success Metrics

- 테마 선택 시 사용자가 색상을 미리 확인할 수 있음
- 커서 이동 → preview 갱신 지연 50ms 이내 (캐시 적중 시 즉시)
- TUI 진입/종료 시 터미널 상태 100% 복원

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-09-theme-preview-brainstorm.md](docs/brainstorms/2026-03-09-theme-preview-brainstorm.md)
  - Key decisions: ratatui+crossterm, single-select, target별 preview 차별화, 고정 ANSI 매핑, Truecolor 우선

### Internal References

- Pipeline orchestration: `src/main.rs` (8-stage pipeline)
- Current theme selection: `src/interactive.rs:select_themes()`
- ThemeIR / HexColor: `src/ir.rs`
- Theme conversion: `src/converter.rs:convert()`
- Theme JSON reading: `src/reader.rs:read_theme_json()`
- Include path (security): `src/reader.rs:read_theme_json_with_includes()` line 157
- Target-specific output: `src/target/ghostty.rs`, `src/target/warp.rs`, `src/target/superset.rs`
- Config diff (ANSI output pattern): `src/target/mod.rs:print_config_diff()`
- Test fixtures: `src/ir.rs:test_fixtures::make_test_ir()`
- Cancellation pattern: `src/interactive.rs:handle_inquire_error()`

### External References

- [ratatui TEA Architecture](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
- [ratatui List Widget](https://ratatui.rs/examples/widgets/list)
- [ratatui Panic Hooks](https://ratatui.rs/recipes/apps/panic-hooks/)
- [ratatui Testing with Insta](https://ratatui.rs/recipes/testing/snapshots/)
- [crossterm Event Handling](https://github.com/crossterm-rs/crossterm)
- [NO_COLOR Convention](https://no-color.org/)

### Conventions

- Conventional commits: `feat:`, `fix:`, `chore:`
- Version bump: feat → minor (0.3.1 → 0.4.0)
- Quality gates: `cargo test && cargo fmt --check && cargo clippy --all-targets`
