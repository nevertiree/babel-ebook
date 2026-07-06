# BabelEbook

[![CI][ci-badge]][ci-url]
[![License: MIT][license-badge]][license-url]
[![Rust Version][rust-badge]][rust-url]
[![Release][release-badge]][release-url]

**BabelEbook**는 대규모 언어 모델(LLM)을 활용한 EPUB 번역기입니다.
원문과 번역문이 함께 있는 이중 언어 전자책을 만들며,
각 번역 단락 뒤에 원문이 따라옵니다.

다른 언어로 읽기: [简体中文](README.md)

[ci-badge]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/badge/License-MIT-yellow.svg
[license-url]: ../LICENSE
[rust-badge]: https://img.shields.io/badge/rust-1.88%2B-blue.svg
[rust-url]: https://www.rust-lang.org/
[release-badge]: https://img.shields.io/github/v/release/nevertiree/babel-ebook
[release-url]: https://github.com/nevertiree/babel-ebook/releases

[screenshot-main]: assets/screenshots/01-translate.png
[screenshot-compute]: assets/screenshots/02-settings-compute.png
[screenshot-translation]: assets/screenshots/03-settings-translation.png
[screenshot-progress]: assets/screenshots/06-translate-progress.png
[screenshot-logs]: assets/screenshots/07-logs-progress.png

> EPUB 콘텐츠와 API 키는 사용자의 컴퓨터에서만 처리되며,
> 프로젝트 관리자의 서버로 전송되지 않습니다.
>
> 다른 언어로 읽기:
> [中文](README.md) · [English](README.en.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Русский](README.ru.md)
> [Español](README.es.md)

<p align="center">
  <img src="assets/screenshots/01-translate.png" alt="BabelEbook 메인 창" width="800">
</p>

<p align="center">
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest/download/BabelEbook_0.1.0_x64-setup.exe">
    <img alt="Windows 다운로드" src="https://img.shields.io/badge/Windows-Download-blue?logo=windows&logoColor=white">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest/download/BabelEbook_0.1.0_amd64.AppImage">
    <img alt="Linux 다운로드" src="https://img.shields.io/badge/Linux-Download-orange?logo=linux&logoColor=white">
  </a>
</p>

---

## BabelEbook을 선택하는 이유

| 기능 | BabelEbook | 온라인 번역기 | Calibre 플러그인 |
|---------|------------|--------------------|-----------------|
| 완전 로컬: EPUB 업로드 없음 | ✅ | ❌ | ✅ |
| 양국어 병렬 배치 | ✅ | 부분 지원 | 수동 조정 필요 |
| 원클릭 데스크톱 설치 프로그램 | ✅ | 설치 불필요 | Calibre 필요 |
| DeepSeek / OpenAI / Anthropic / Ollama | ✅ | 고정 벤더 | 플러그인 의존 |
| 용어집, 제외 선택자, 동시 처리 | ✅ | 부분 지원 | 플러그인 의존 |

---

## 스크린샷

| 메인 창 | 컴퓨팅 설정 | 번역 옵션 |
|-------------|------------------|---------------------|
| ![메인 창][screenshot-main] | ![컴퓨팅 설정][screenshot-compute] | ![번역 옵션][screenshot-translation] |

| 번역 진행 상황 | 로그 |
|----------------------|------|
| ![번역 진행 상황][screenshot-progress] | ![로그][screenshot-logs] |

---

## 지원 플랫폼

데스크톱 GUI는 다음 운영체제에서 사용할 수 있습니다.

- **Windows** (권장): `.exe`(NSIS) 및 `.msi` 설치 파일.
- **Linux**: `.AppImage`(포터블, 더블클릭 실행)와 Debian/Ubuntu 기반 배포판용
  `.deb` 패키지.

macOS에는 공식 데스크톱 설치 프로그램이 **없습니다**.
macOS 사용자는 소스에서 명령줄 버전을 직접 빌드해 실행할 수 있습니다.

---

## 사용자 가이드

### 다운로드 및 설치

1. [Releases](https://github.com/nevertiree/babel-ebook/releases) 페이지를 엽니다.
2. 사용 중인 시스템에 맞는 설치 파일을 다운로드합니다.

   **Windows**
   - **대부분의 사용자에게 권장**: `BabelEbook_<version>_x64-setup.exe`
     (NSIS 설치 프로그램, 시스템 언어에 맞춰 자동 전환).
   - **IT 관리자 또는 자동 배포**: `BabelEbook_<version>_x64_en-US.msi`
     (MSI 설치 프로그램).

   **Linux**
   - **대부분의 배포판에 권장**: `BabelEbook_<version>_amd64.AppImage`
     (설치 불필요; `chmod +x` 실행 후 더블클릭).
   - **Debian / Ubuntu**: `BabelEbook_<version>_amd64.deb`
     (더블클릭으로 설치하거나 `sudo dpkg -i BabelEbook_<version>_amd64.deb` 실행).

3. 설치 파일을 더블클릭하고 안내에 따라 진행합니다.

> **Linux에서 중국어 글꼴 표시:** Linux 시스템에 중국어 글꼴이 설치되어 있지 않으면
> UI의 중국어 문자가 사각형으로 표시될 수 있습니다.
> Debian/Ubuntu에서는 `fonts-noto-cjk` 같은 시스템 권장 중국어 글꼴 패키지를 설치하세요.
>
> ```bash
> sudo apt-get install fonts-noto-cjk
> ```

### 첫 사용

#### 1. API 키 준비

BabelEbook은 서드파티 대규모 언어 모델 API를 호출해야 합니다.
현재 DeepSeek, OpenAI, Anthropic, 로컬에서 실행하는 Ollama를 지원합니다.

DeepSeek을 예로 들면:

1. [DeepSeek 플랫폼](https://platform.deepseek.com/)에 접속해 가입하고 API 키를 만듭니다.
2. BabelEbook을 실행하고 **설정** → **컴퓨팅**으로 이동합니다.
3. 제공자로 `DeepSeek`을 선택하고 API 키를 입력합니다.
4. **연결 테스트**를 클릭해 연결을 확인합니다.

> 로컬 Ollama를 사용할 때는 API 키가 필요 없으며,
> Base URL(예: `http://localhost:11434`)만 입력하면 됩니다.

### 책 번역하기

1. 메인 화면에서 **EPUB 선택**을 클릭해 번역할 전자책을 선택합니다.
2. 대상 언어를 선택합니다(기본값은 간체 중국어 `zh-CN`).
3. **번역 시작**을 클릭합니다.
4. 출력 파일은 지정한 위치에 저장됩니다.

### 일반 설정

| 설정 | 설명 |
|---------|-------------|
| 제공자 / API | LLM 제공자를 선택하고 API 키를 입력합니다. |
| 대상 언어 | 번역 대상 언어입니다. 예: `zh-CN`, `en`, `ja` 등. |
| 출력 모드 | `bilingual`(원문 + 번역), `translation_only`(번역문만), `interleaved`(단락 교차). |
| 동시 처리 | 병렬로 번역할 장 수입니다. 값이 높을수록 빠르지만 비용도 늘어납니다. |
| 최대 입력/출력 토큰 | 요청당 최대 토큰 수입니다. 기본값을 권장합니다. |
| 제외 선택자 | 걸러낼 요소입니다. 예: `.code`, `pre`. |
| 용어집 | 고유 명사 번역을 고정하는 용어표입니다. |

### 출력 모드

- **Bilingual**: 번역된 각 단락 뒤에 원문이 따라옵니다. 언어 학습에 적합합니다.
- **Translation only**: 번역문만 남깁니다.
- **Interleaved**: 원문과 번역문이 번갈아 나옵니다.

### UI 언어

데스크톱 앱은 English, Español, 日本語, 한국어, Русский, 简体中文를 지원합니다.
UI 언어는 첫 실행 시 시스템 언어를 기준으로 자동 선택되며, 설정에서 변경할 수 있습니다.

### 자주 묻는 질문

**Q: 번역 결과가 비어 있거나 장이 누락되었습니다.**
A: EPUB 콘텐츠가 스캔 이미지인지 확인하고, 그렇다면 먼저 OCR을 실행하세요.
`Exclude Selectors`를 조정해 번역하지 않을 요소를 걸러낼 수도 있습니다.

**Q: 번역에 토큰이 얼마나 소모되나요?**
A: 메인 화면이나 CLI의 **Dry Run** 모드를 사용하면 실제 API 호출 없이 토큰 수를 확인할 수 있습니다.

**Q: API 키가 안전한가요?**
A: 네. API 키는 기본적으로 Windows Credential Manager에 저장되며,
평문 설정 파일에는 저장되지 않습니다.

---

## 개발자 가이드

### 프로젝트 소개

BabelEbook은 Rust + TypeScript 아키텍처를 사용합니다.

- **Rust core** (`crates/babel-ebook`): EPUB 파싱, 청킹, 캐싱, LLM 호출.
- **Rust CLI** (`crates/babel-ebook-cli`): 명령줄 진입점.
- **Tauri desktop app** (`desktop/`): Rust 백엔드 + React/TypeScript 프론트엔드.

### 요구 사항

- [Rust](https://rustup.rs/) 1.88 이상
- [pnpm](https://pnpm.io/) 9+ (데스크톱 개발)
- Windows 10/11 (데스크톱 GUI 개발용)
- 선택한 제공자의 API 키

### 빠른 시작

```bash
# 저장소 복제
git clone https://github.com/nevertiree/babel-ebook.git
cd babel-ebook

# Rust 워크스페이스 빌드 및 테스트
cargo build --workspace
cargo test --workspace

# 데스크톱 프론트엔드 의존성 설치
cd desktop
pnpm install

# 데스크톱 개발 서버 시작
pnpm tauri dev
```

### 프로젝트 구조

```text
├── Cargo.toml              # 워크스페이스 버전(단일 진실 공급원)
├── crates/
│   ├── babel-ebook/        # 핵심 번역 라이브러리(Rust)
│   └── babel-ebook-cli/    # 명령줄 인터페이스(Rust)
├── desktop/
│   ├── src/                # React + i18next 프론트엔드(TypeScript)
│   ├── src-tauri/          # Tauri Rust 백엔드
│   ├── e2e/                # Playwright GUI 테스트
│   └── scripts/            # 빌드 및 릴리스 헬퍼
└── release/v<x.y.z>/       # 최종 배포용 설치 파일(생성됨)
```

### 빌드 명령

#### CLI

```bash
cargo build --release -p babel-ebook-cli
# Output: target/release/babel-ebook
```

#### Windows 데스크톱 설치 파일

```bash
cd desktop
pnpm install
pnpm tauri build
```

출력물:

- MSI: `target/release/bundle/msi/BabelEbook_<version>_x64_en-US.msi`
- NSIS: `target/release/bundle/nsis/BabelEbook_<version>_x64-setup.exe`

#### Linux 데스크톱 설치 파일

Debian/Ubuntu 또는 호환 배포판에서 먼저 Tauri 의존성을 설치합니다.

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev xdg-utils
```

그런 다음 빌드합니다.

```bash
cd desktop
pnpm install
pnpm tauri build
```

출력물:

- AppImage: `target/release/bundle/appimage/BabelEbook_<version>_amd64.AppImage`
- deb: `target/release/bundle/deb/BabelEbook_<version>_amd64.deb`

> **Linux에서 중국어 UI 글꼴:** Linux 시스템에 중국어 글꼴이 설치되어 있지 않으면
> UI의 중국어 문자가 사각형으로 표시될 수 있습니다.
> `fonts-noto-cjk`(Debian/Ubuntu: `sudo apt-get install fonts-noto-cjk`) 또는
> 다른 시스템 중국어 글꼴을 설치하세요.

### 품질 기준

PR을 열기 전에 다음 항목이 통과하는지 확인하세요.

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd desktop
pnpm exec tsc --noEmit
pnpm build
```

### 기여 가이드

기여를 환영합니다!
먼저 [.github/CONTRIBUTING.md](.github/CONTRIBUTING.md),
[.github/CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md),
[.github/SECURITY.md](.github/SECURITY.md)를 읽어 주세요.

#### 브랜치 모델

이 프로젝트는 **Git Flow**를 따릅니다.

- `master`: 출시된 프로덕션 코드.
- `develop`: 일일 통합 기준.
- `release/<version>`: 릴리스 안정화 브랜치.
- `feature/<name>`: 기능 브랜치.

#### 커밋 스타일

- [Conventional Commits](https://www.conventionalcommits.org/) 사용:
  - `feat:` 새로운 기능
  - `fix:` 버그 수정
  - `docs:` 문서 업데이트
  - `refactor:` 리팩터링
  - `chore:` 빌드/도구/기타
- 커밋은 작고 목적에 맞게 유지하세요.
- API 키, 개인 경로, 내부 계획 문서는 커밋하지 마세요.

#### PR 요구 사항

1. 모든 CI 검사를 통과해야 합니다.
2. 사용자에게 영향을 주는 동작이 변경되면 `docs/README.md`와 `CHANGELOG.md`를 업데이트하세요.
3. diff는 기능 또는 수정 범위로 제한하세요.
4. 데스크톱 변경 사항은 Playwright E2E 테스트를 포함하거나 업데이트해야 합니다.

### 릴리스 워크플로

```bash
cd desktop

# 1. 버전 올리기(patch / minor / major), Cargo.toml/package.json/tauri.conf.json 동기화 및 태그 생성
pnpm version:bump minor

# 2. 태그 커밋에서 전체 빌드 실행
pnpm release:build
```

최종 산출물은 `release/v<version>/`에 복사됩니다.

### 고급 CLI 사용법

```bash
export DEEPSEEK_API_KEY=sk-...

cargo run --release -p babel-ebook-cli -- input.epub -o output.epub \
  --provider deepseek \
  --model deepseek-chat \
  --concurrency 3 \
  --max-input-tokens 4000 \
  --max-output-tokens 2000

# API를 호출하지 않고 토큰만 예측
 cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --dry-run

# JSON 설정 파일 사용
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --config config.json
```

전체 CLI 인수 목록은 `babel-ebook --help`를 실행해 확인하세요.

### 지원하는 LLM 제공자

| 제공자 | `--provider` | 기본 모델 | 기본 URL | 참고 |
|----------|--------------|---------------|----------|-------|
| DeepSeek | `deepseek` | `deepseek-chat` | `https://api.deepseek.com` | 권장 기본값 |
| OpenAI | `openai` | — | `https://api.openai.com/v1` | `--model` 명시 필요 |
| Anthropic | `anthropic` | `claude-3-5-sonnet-20241022` | `https://api.anthropic.com` | — |
| Ollama | `ollama` | `llama3` | local | API 키 불필요 |
| OpenAI-compatible | `openai-compatible` | — | `base_url`로 설정 | 자체 호스팅 또는 프록시 엔드포인트용 |

### 보안

- **API 키를 절대 커밋하지 마세요.**
  - 환경 변수, OS 키링, 또는 `.gitignore`로 무시된 로컬 설정 파일을 사용하세요.
  - 코드에 API 키를 작성하거나 Git에 커밋하지 마세요.
- 보안 취약점은 [.github/SECURITY.md](.github/SECURITY.md)를 통해 비공개로 신고해 주세요.

### 감사의 글

Rust, Tauri, React, i18next로 제작되었습니다.

## 라이선스

MIT
